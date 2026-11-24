//! Radar geometry. Deterministic: the same data always yields the same picture.
//!
//! Coordinates are SVG user units, origin at the centre, y pointing down. The radar has
//! radius `R` inside a square viewBox of half-size `VB`; the corners hold quadrant labels.

use crate::model::Corner;
use crate::view::{RadarView, State};
use std::cell::RefCell;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::fmt::Write;

pub const R: f64 = 360.0;
pub const VB: f64 = 400.0;
/// Outer radius of each ring, Adopt to Hold. Adopt is the widest: it holds the defaults, which
/// on a working radar are the most entries (21 of 41 in edition 01), and its labels need room.
pub const RING_OUTER: [f64; 4] = [185.0, 262.0, 316.0, 360.0];
pub const BLIP_R: f64 = 8.0;
/// Minimum centre-to-centre distance between two blips.
const MIN_GAP: f64 = 30.0;
const PAD_DEG: f64 = 8.0;
/// Martian Mono at 75% width (radar labels): every glyph advances 0.6em.
pub const MONO_ADVANCE: f64 = 0.6;

pub fn band(k: usize) -> (f64, f64) {
    (if k == 0 { 0.0 } else { RING_OUTER[k - 1] }, RING_OUTER[k])
}

/// The part of a band where blips may sit (keeps them off the centre point).
fn place_band(k: usize) -> (f64, f64) {
    let (a, b) = band(k);
    (if k == 0 { 42.0 } else { a + 4.0 }, b - 4.0)
}

/// Start angle (radians, clockwise from +x because y points down) of a quadrant's 90° sweep.
pub fn start_angle(c: Corner) -> f64 {
    (match c {
        Corner::BottomRight => 0.0,
        Corner::BottomLeft => 90.0,
        Corner::TopLeft => 180.0,
        Corner::TopRight => 270.0,
    }) * PI
        / 180.0
}

/// Unit direction (sign of x, sign of y) pointing into a corner.
pub fn corner_sign(c: Corner) -> (f64, f64) {
    match c {
        Corner::TopLeft => (-1.0, -1.0),
        Corner::TopRight => (1.0, -1.0),
        Corner::BottomLeft => (-1.0, 1.0),
        Corner::BottomRight => (1.0, 1.0),
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Pos {
    pub x: f64,
    pub y: f64,
    pub theta: f64,
    /// Radial position within the placement band, 0 = inner edge, 1 = outer edge.
    pub frac: f64,
}

pub fn point(theta: f64, ring: usize, frac: f64) -> (f64, f64) {
    let (a, b) = place_band(ring);
    let r = a + (b - a) * frac;
    (r * theta.cos(), r * theta.sin())
}

/// Label sizes in user units: overview, zoomed desktop, zoomed phone (site.css `.lbl`).
pub const F_OVER: f64 = 12.5;
pub const F_ZOOM: f64 = 8.5;
pub const F_ZOOM_M: f64 = 14.0;
/// Clearance radius around a blip: the dot (r 8 plus half its 2.5 stroke) or the halo of a
/// new blip (r 13 plus half its 2 stroke), each with one unit to spare.
pub const R_DOT: f64 = 10.25;
pub const R_HALO: f64 = 15.0;
/// Blip scale in the zoomed view (site.css `.zoomed .blip .dot`).
pub const Z_BLIP: f64 = 0.72;

/// Where a quadrant's overview labels may go: its part of the scope, clear of the quadrant
/// titles in the corners.
pub fn overview_bounds(c: Corner) -> BoxF {
    let (sx, sy) = corner_sign(c);
    let (x0, x1) = if sx < 0.0 { (-VB + 2.0, 0.0) } else { (0.0, VB - 2.0) };
    let (y0, y1) = if sy < 0.0 { (-VB + 46.0, 0.0) } else { (0.0, VB - 46.0) };
    BoxF { x0, y0, x1, y1 }
}

/// Where a zoomed quadrant's labels may go: the quadrant square the zoom shows, inset a little.
pub fn zoom_bounds(c: Corner, font: f64) -> BoxF {
    let (sx, sy) = corner_sign(c);
    let pad = font * 0.6;
    let (x0, x1) = if sx < 0.0 { (-VB, 0.0) } else { (0.0, VB) };
    let (y0, y1) = if sy < 0.0 { (-VB, 0.0) } else { (0.0, VB) };
    BoxF { x0: x0 + pad, y0: y0 + pad, x1: x1 - pad, y1: y1 - pad }
}

/// Places one ring cell: `m` zig-zag rows, the first blip on row `phase` counted from the
/// outside, in name order or, with `rev`, backwards. `lift` (0 to 1) keeps the rows off the
/// inner part of the band, where there is least room. `shift` turns the cell by a share of the
/// angle between neighbours.
fn place_cell(cell: &[usize], k: usize, corner: Corner, (m, phase, lift, rev, shift): Var, out: &mut [Option<Pos>]) {
    let n = cell.len() as f64;
    let pad = PAD_DEG * PI / 180.0;
    let dt = (PI / 2.0 - 2.0 * pad) / n;
    let fr: Vec<f64> = rows(m, lift);
    let t0 = start_angle(corner) + pad;
    for (j, &i) in cell.iter().enumerate() {
        let j = if rev { cell.len() - 1 - j } else { j };
        let theta = t0 + dt * (j as f64 + 0.5 + shift);
        let frac = fr[(m - 1) - ((j + phase) % m)];
        let (x, y) = point(theta, k, frac);
        out[i] = Some(Pos { x, y, theta, frac });
    }
}

/// A cell layout: rows, zig-zag phase, lift, whether the name order runs backwards, and turn.
type Var = (usize, usize, f64, bool, f64);
const LIFTS: [f64; 3] = [0.0, 0.3, 0.55];
const SHIFTS: [f64; 3] = [0.0, -0.3, 0.3];
/// Most zig-zag rows in one cell.
const MAX_ROWS: usize = 4;

/// Radial positions of `m` rows as fractions of the band.
fn rows(m: usize, lift: f64) -> Vec<f64> {
    (0..m).map(|j| lift + (1.0 - lift) * (j as f64 + 0.5) / m as f64).collect()
}

/// Ring names sit on the horizontal axis at the middle of each band, at most six letters
/// wide: the furniture the renderer draws there (see `svg::furniture_boxes`).
pub fn axis_marks(font: f64) -> Vec<BoxF> {
    (0..4)
        .flat_map(|k| {
            let (a, b) = band(k);
            let w = 6.0 * font * 0.79;
            [1.0, -1.0].map(|s| BoxF { x0: s * (a + b) / 2.0 - w / 2.0, y0: -6.0 - ASC * font, x1: s * (a + b) / 2.0 + w / 2.0, y1: -6.0 + DESC * font }.grow(1.5, 1.0))
        })
        .collect()
}

/// The two axis lines, drawn out to the frame: an overview label never crosses into a
/// neighbouring quadrant. Labels keep one font size from the vertical axis, so two labels on
/// either side of it keep the side gap of `labels_clash`. Each quadrant's labels can then be
/// placed on their own.
pub fn axis_lines(font: f64) -> [BoxF; 2] {
    [BoxF { x0: -VB, y0: -1.0, x1: VB, y1: 1.0 }, BoxF { x0: -font, y0: -VB, x1: font, y1: VB }]
}

/// How well labels fit with a quadrant laid out one way, compared in this order: labels in the
/// overview, labels in its zoomed view, labels in its zoomed phone view, the fewest overview
/// labels with a leader line, the shortest leaders in total, and the slack: free spots of the
/// labels left out, so a search can move towards a layout where they fit.
type Fit = (usize, usize, usize, std::cmp::Reverse<usize>, std::cmp::Reverse<usize>, usize);

/// How many overview labels need a leader, and the leaders' total length.
fn leads(over: &[Label]) -> (std::cmp::Reverse<usize>, std::cmp::Reverse<usize>) {
    let ls: Vec<Lead> = over.iter().filter_map(|l| l.lead).collect();
    (std::cmp::Reverse(ls.len()), std::cmp::Reverse(ls.iter().map(|s| (s[2] - s[0]).hypot(s[3] - s[1])).sum::<f64>() as usize))
}

/// Labels of quadrant `q`: in the overview (`zoom` None, where each quadrant's labels are
/// independent) or in its zoomed view at one font size. `orders` limits the label orders tried
/// (see `place_labels`): a layout search needs a quick count, the page the best result.
fn quadrant_labels(rv: &RadarView, q: usize, corner: Corner, e: usize, pos: &[Option<Pos>], zoom: Option<f64>, orders: usize) -> (Vec<Label>, usize) {
    let r = |i: usize| if rv.blips[i].state_at(e) == State::New { R_HALO } else { R_DOT };
    let scale = if zoom.is_some() { Z_BLIP } else { 1.0 };
    let items: Vec<LabelItem> = (0..pos.len())
        .filter(|&i| zoom.is_none() || rv.blips[i].q == q)
        .filter_map(|i| {
            let p = pos[i]?;
            let required = matches!(rv.blips[i].state_at(e), State::New | State::In | State::Out);
            let text = if rv.blips[i].q == q { scope_text(rv.blips[i].e) } else { String::new() };
            Some(LabelItem { blip: i, x: p.x, y: p.y, r: r(i) * scale, text, required })
        })
        .collect();
    let (font, bounds) = match zoom {
        Some(font) => (font, zoom_bounds(corner, font)),
        None => (F_OVER, overview_bounds(corner)),
    };
    let mut obs: Vec<BoxF> = axis_marks(if zoom.is_some() { 6.0 } else { 10.0 });
    if zoom.is_none() {
        obs.extend(axis_lines(font));
    }
    obs.extend(pos.iter().enumerate().filter_map(|(i, p)| p.map(|p| BoxF::around(p.x, p.y, r(i) * scale))));
    // The scope draws trails at the latest edition only.
    if e == rv.latest() {
        obs.extend(trail_obstacles(rv, pos));
    }
    place_labels_with(&items, &obs, font, bounds, orders)
}

/// Label orders a layout search tries: the five sorted ones.
const QUICK: usize = 5;

fn label_fit(rv: &RadarView, q: usize, corner: Corner, e: usize, pos: &[Option<Pos>]) -> Fit {
    fit_with(rv, q, corner, e, pos, &quadrant_labels(rv, q, corner, e, pos, None, QUICK))
}

fn fit_with(rv: &RadarView, q: usize, corner: Corner, e: usize, pos: &[Option<Pos>], (over, slack): &(Vec<Label>, usize)) -> Fit {
    let (n, len) = leads(over);
    let zoom = |font: f64| quadrant_labels(rv, q, corner, e, pos, Some(font), QUICK).0.len();
    (over.len(), zoom(F_ZOOM), zoom(F_ZOOM_M), n, len, *slack)
}

/// The fit of a layout when it beats `best`. The overview comes first, so the zoomed views are
/// only counted when the overview does at least as well. When `best` already shows every label
/// in both zoomed views, they cannot do better, so the overview alone decides.
fn fit_if_better(rv: &RadarView, q: usize, corner: Corner, e: usize, pos: &[Option<Pos>], best: &Fit) -> Option<Fit> {
    let over = quadrant_labels(rv, q, corner, e, pos, None, QUICK);
    if over.0.len() < best.0 {
        return None;
    }
    let nq = rv.blips.iter().zip(pos).filter(|(b, p)| b.q == q && p.is_some()).count();
    let (n, len) = leads(&over.0);
    if over.0.len() == best.0 && best.1 == nq && best.2 == nq && (n, len, over.1) <= (best.3, best.4, best.5) {
        return None;
    }
    Some(fit_with(rv, q, corner, e, pos, &over)).filter(|s| s > best)
}

/// Blips keep this far from the axes, so no dot touches an axis line.
const AXIS_CLEAR: f64 = R_DOT + 2.0;

/// Moves single blips to free points of their own cell while more labels then fit. Blips whose
/// label is missing or needs a leader go first, then their neighbours, nearest first. A point
/// is free when it keeps `MIN_GAP` from every other blip.
fn refine(rv: &RadarView, q: usize, corner: Corner, e: usize, out: &mut [Option<Pos>], best: &mut Fit) {
    let nq = rv.blips.iter().zip(out.iter()).filter(|(b, p)| b.q == q && p.is_some()).count();
    let pad = PAD_DEG * PI / 180.0;
    for _ in 0..3 {
        if best.0 == nq && best.3.0 == 0 {
            return;
        }
        let (over, _) = quadrant_labels(rv, q, corner, e, out, None, QUICK);
        let weak: Vec<usize> = (0..out.len()).filter(|&i| rv.blips[i].q == q && out[i].is_some() && over.iter().find(|l| l.blip == i).is_none_or(|l| l.lead.is_some())).collect();
        let at = |i: usize| out[i].map(|p| (p.x, p.y)).unwrap();
        let mut movers: Vec<(f64, usize)> = (0..out.len())
            .filter(|&i| rv.blips[i].q == q && out[i].is_some())
            .map(|i| (weak.iter().map(|&w| (at(i).0 - at(w).0).hypot(at(i).1 - at(w).1)).fold(f64::INFINITY, f64::min), i))
            .filter(|(d, _)| *d < 90.0)
            .collect();
        movers.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut improved = false;
        for (_, i) in movers {
            let p0 = out[i];
            let k = rv.blips[i].rings[e].unwrap();
            let mut pick = None;
            for step in 0..=24 {
                let theta = start_angle(corner) + pad + (PI / 2.0 - 2.0 * pad) * step as f64 / 24.0;
                for frac in [0.0, 0.2, 0.4, 0.6, 0.8, 1.0] {
                    let (x, y) = point(theta, k, frac);
                    if x.abs() < AXIS_CLEAR || y.abs() < AXIS_CLEAR || out.iter().enumerate().any(|(j, p)| j != i && p.is_some_and(|p| (p.x - x).hypot(p.y - y) < MIN_GAP)) {
                        continue;
                    }
                    out[i] = Some(Pos { x, y, theta, frac });
                    if let Some(s) = fit_if_better(rv, q, corner, e, out, best) {
                        *best = s;
                        pick = out[i];
                    }
                }
            }
            out[i] = pick.or(p0);
            improved |= pick.is_some();
        }
        if !improved {
            return;
        }
    }
}

/// The text a blip's label shows on the scope: its short label or its name, in the entry's own
/// case, so the label keeps the word shapes of the name in the lists.
pub fn scope_text(e: &crate::model::Entry) -> String {
    e.scope_name().to_string()
}

/// Positions of every blip at edition `e` (None when absent).
///
/// Each ring cell spreads its blips evenly by angle in zig-zag rows. The fewest rows that keep
/// neighbours apart come first; more rows, a shifted zig-zag or a turned cell replace them only
/// when labels then fit better (see `Fit`). Where a label still finds no spot or needs a leader,
/// single blips then move to free points of their cell (see `refine`).
pub fn layout(rv: &RadarView, corners: &[Corner], e: usize) -> Vec<Option<Pos>> {
    // Pages, cards and the replay ask for the same layouts many times: compute each once.
    // The key holds everything the layout depends on.
    let mut key = format!("{corners:?}");
    for b in &rv.blips {
        let _ = write!(key, "|{}:{}:{:?}:{}", b.e.id, b.q, b.rings[e], b.state_at(e) == State::New);
    }
    thread_local! {
        static CACHE: RefCell<HashMap<String, Vec<Option<Pos>>>> = RefCell::new(HashMap::new());
    }
    if let Some(hit) = CACHE.with(|c| c.borrow().get(&key).cloned()) {
        return hit;
    }
    let out = layout_uncached(rv, corners, e);
    CACHE.with(|c| c.borrow_mut().insert(key, out.clone()));
    out
}

fn layout_uncached(rv: &RadarView, corners: &[Corner], e: usize) -> Vec<Option<Pos>> {
    let mut out = vec![None; rv.blips.len()];
    for (q, &corner) in corners.iter().enumerate() {
        let mut cells: Vec<(usize, Vec<usize>, Vec<Var>)> = Vec::new();
        for k in 0..4 {
            // Blips are pre-sorted by name, so cells are too.
            let cell: Vec<usize> = rv.blips.iter().enumerate().filter(|(_, b)| b.q == q && b.rings[e] == Some(k)).map(|(i, _)| i).collect();
            if cell.is_empty() {
                continue;
            }
            let dt = (PI / 2.0 - 2.0 * PAD_DEG * PI / 180.0) / cell.len() as f64;
            let (a, b) = place_band(k);
            let fits = |m: usize, lift: f64| {
                let fr = rows(m, lift);
                let rmin = a + (b - a) * fr[0];
                let dr = if m > 1 { (b - a) * (fr[1] - fr[0]) } else { 0.0 };
                rmin * dt * m as f64 >= MIN_GAP && (m == 1 || ((rmin * dt).powi(2) + dr * dr).sqrt() >= MIN_GAP)
            };
            let m0 = (1..=3).find(|&m| fits(m, 0.0)).unwrap_or(3);
            // Variants: the default first, then more rows, shifted zig-zags, lifted rows and turns.
            let mut vars: Vec<Var> = vec![(m0, 0, 0.0, false, 0.0)];
            for shift in SHIFTS {
                for rev in [false, true] {
                    for lift in LIFTS {
                        for m in m0..=MAX_ROWS.min(cell.len()) {
                            for ph in 0..m {
                                let v = (m, ph, lift, rev, shift);
                                if v != vars[0] && fits(m, lift) && (cell.len() > 1 || !rev) {
                                    vars.push(v);
                                }
                            }
                        }
                    }
                }
            }
            place_cell(&cell, k, corner, vars[0], &mut out);
            cells.push((k, cell, vars));
        }
        // Try the cell variants, one cell at a time, while that frees room for labels.
        let mut best = label_fit(rv, q, corner, e, &out);
        let nq = rv.blips.iter().zip(&out).filter(|(b, p)| b.q == q && p.is_some()).count();
        for _ in 0..2 {
            // Every label of this quadrant shows in the overview and both zoomed views, without
            // a leader: nothing to win.
            if best.0 == nq && best.1 == nq && best.2 == nq && best.3.0 == 0 {
                break;
            }
            let mut changed = false;
            for (k, cell, vars) in &cells {
                let keep: Vec<Option<Pos>> = cell.iter().map(|&i| out[i]).collect();
                let mut pick: Option<Var> = None;
                for &v in &vars[1..] {
                    place_cell(cell, *k, corner, v, &mut out);
                    if cell.iter().any(|&i| out[i].is_some_and(|p| p.x.abs() < AXIS_CLEAR || p.y.abs() < AXIS_CLEAR)) {
                        continue;
                    }
                    if let Some(s) = fit_if_better(rv, q, corner, e, &out, &best) {
                        best = s;
                        pick = Some(v);
                    }
                }
                match pick {
                    Some(v) => {
                        place_cell(cell, *k, corner, v, &mut out);
                        changed = true;
                    }
                    None => {
                        for (&i, p) in cell.iter().zip(keep) {
                            out[i] = p;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        refine(rv, q, corner, e, &mut out, &mut best);
    }
    out
}

/// Trails, sampled along each segment, so labels avoid crossing them. The layout search counts
/// them too, so the labels it finds room for are the ones the scope can draw.
pub fn trail_obstacles(rv: &RadarView, pos: &[Option<Pos>]) -> Vec<BoxF> {
    let mut v: Vec<BoxF> = Vec::new();
    for (i, p) in pos.iter().enumerate() {
        let Some(p) = p else { continue };
        let t = trail(rv, i, *p, 3);
        for w in t.windows(2) {
            for s in 0..=8 {
                let f = s as f64 / 8.0;
                v.push(BoxF::around(w[0].0 + (w[1].0 - w[0].0) * f, w[0].1 + (w[1].1 - w[0].1) * f, 5.0));
            }
        }
    }
    v
}

/// Past positions a blip moved through within the last `window` editions, oldest first,
/// placed at its current angle so the trail reads as a radial movement.
pub fn trail(rv: &RadarView, i: usize, now: Pos, window: usize) -> Vec<(f64, f64, usize)> {
    let b = &rv.blips[i];
    let last = rv.latest();
    let from = last.saturating_sub(window);
    let mut pts: Vec<(f64, f64, usize)> = Vec::new();
    let mut prev: Option<usize> = None;
    for e in from..=last {
        if let Some(k) = b.rings[e]
            && prev != Some(k)
        {
            let (x, y) = point(now.theta, k, now.frac);
            pts.push((x, y, e));
            prev = Some(k);
        }
    }
    if pts.len() < 2 { Vec::new() } else { pts }
}

/// Em box of a radar label above and below its baseline, as a share of the font size.
/// Chrome's getBBox on Martian Mono gives at most 1.036 above and 0.222 below (at 7px, where
/// it rounds to whole device pixels).
pub const ASC: f64 = 1.05;
pub const DESC: f64 = 0.24;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxF {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl BoxF {
    pub fn around(x: f64, y: f64, r: f64) -> BoxF {
        BoxF { x0: x - r, y0: y - r, x1: x + r, y1: y + r }
    }
    pub fn hits(&self, o: &BoxF) -> bool {
        self.x0 < o.x1 && o.x0 < self.x1 && self.y0 < o.y1 && o.y0 < self.y1
    }
    pub fn grow(&self, dx: f64, dy: f64) -> BoxF {
        BoxF { x0: self.x0 - dx, y0: self.y0 - dy, x1: self.x1 + dx, y1: self.y1 + dy }
    }
    pub fn inside(&self, b: &BoxF) -> bool {
        self.x0 >= b.x0 && self.y0 >= b.y0 && self.x1 <= b.x1 && self.y1 <= b.y1
    }
}

/// The box a label occupies when drawn: the same metric for placement and for tests.
pub fn text_box(x: f64, y: f64, anchor: &str, text: &str, font: f64) -> BoxF {
    let w = text.chars().count() as f64 * font * MONO_ADVANCE;
    let x0 = match anchor {
        "end" => x - w,
        "middle" => x - w / 2.0,
        _ => x,
    };
    BoxF { x0, y0: y - ASC * font, x1: x0 + w, y1: y + DESC * font }
}

/// True when two labels sit too close. Side by side they need three characters of space, so
/// two names on one line never read as one name. Stacked they need `STACK_GAP`.
pub fn labels_clash(a: &BoxF, b: &BoxF, font: f64) -> bool {
    a.grow(2.0 * font, STACK_GAP * font).hits(b)
}

/// Space (in font sizes) between two labels stacked one above the other. The lines of one
/// wrapped label touch, so two names never read as one name on two lines.
pub const STACK_GAP: f64 = 0.6;
/// Space (in font sizes) above and below a label that is not level with its blip.
pub const LOOSE_GAP: f64 = 1.0;

/// Extra pseudo-random label orders to try after the five ordered ones.
const SHUFFLES: usize = 12;

/// Baseline distance between the lines of a wrapped label: the em boxes stack without gap.
pub const LINE: f64 = ASC + DESC;

/// A label reads as its own blip's: every other blip is at least this much further (in font
/// sizes) from the label's attachment points than its own blip.
pub const ASSOC: f64 = 1.0;
/// No other blip comes closer than this (in font sizes) to a label.
pub const BODY_GAP: f64 = 0.5;
/// Space (in font sizes) between a leader's label and any other blip's attachment.
pub const LEAD_CLEAR: f64 = 1.0;
/// Space between a leader and any other blip's clearance circle, and between two leaders.
pub const LEAD_GAP: f64 = 3.0;
/// Space between the end of a leader and its label.
const LEAD_END: f64 = 2.0;

/// A leader line from the edge of a blip to its label: x0, y0, x1, y1.
pub type Lead = [f64; 4];

#[derive(Clone, Debug)]
pub struct Label {
    pub blip: usize,
    pub x: f64,
    /// Baseline of the first line.
    pub y: f64,
    pub anchor: &'static str,
    /// One line, or two when the name is long and one line finds no free spot.
    pub lines: Vec<String>,
    /// A short line from the blip to the label, when the label sits a little away from it.
    pub lead: Option<Lead>,
}

/// A blip to label: blip index, centre, clearance radius, the text, and whether it marks a
/// change (changes are placed first). An item with an empty text gets no label, but labels
/// keep clear of it and must read as closer to their own blip than to it.
pub struct LabelItem {
    pub blip: usize,
    pub x: f64,
    pub y: f64,
    pub r: f64,
    pub text: String,
    pub required: bool,
}

/// Distance from a point to a box (0 inside it).
pub fn box_dist(b: &BoxF, x: f64, y: f64) -> f64 {
    (b.x0 - x).max(0.0).max(x - b.x1).hypot((b.y0 - y).max(0.0).max(y - b.y1))
}

/// Where a reader looks for a label's blip: the middle of either end, the middle of the top
/// and bottom edges, and the four corners.
fn attach_points(b: &BoxF) -> [(f64, f64); 8] {
    let (cx, cy) = ((b.x0 + b.x1) / 2.0, (b.y0 + b.y1) / 2.0);
    [(b.x0, cy), (b.x1, cy), (cx, b.y0), (cx, b.y1), (b.x0, b.y0), (b.x1, b.y0), (b.x0, b.y1), (b.x1, b.y1)]
}

/// How far a blip (x, y, clearance r) is from the nearest attachment point of a label box.
pub fn attach_dist(b: &BoxF, x: f64, y: f64, r: f64) -> f64 {
    attach_points(b).iter().map(|(px, py)| (px - x).hypot(py - y)).fold(f64::INFINITY, f64::min) - r
}

/// Depth (in font sizes) of the strip between a label and its blip that no other blip enters.
pub const FACING: f64 = 2.5;

/// The strip on the side of a label that faces its blip (x, y): the label box stretched
/// `FACING` font sizes towards the blip, up, down, left or right, whichever the blip is most.
pub fn facing_zone(b: &BoxF, x: f64, y: f64, font: f64) -> BoxF {
    let d = FACING * font;
    let (dx, dy) = ((b.x0 - x).max(x - b.x1), (b.y0 - y).max(y - b.y1));
    if dy >= dx {
        if y > b.y1 { BoxF { y1: b.y1 + d, ..*b } } else { BoxF { y0: b.y0 - d, ..*b } }
    } else if x < b.x0 {
        BoxF { x0: b.x0 - d, ..*b }
    } else {
        BoxF { x1: b.x1 + d, ..*b }
    }
}

/// Space (in font sizes) beyond either end of a label, level with its text, that no other blip
/// enters. A dot there reads as the label's own: "Guard hooks ●".
pub const END_CLEAR: f64 = 3.0;

/// True when a blip (x, y, clearance r) sits level with a label and close beyond one of its
/// ends: its centre is within a quarter font size of the label's text band, and its edge within
/// `END_CLEAR` font sizes of the label's end.
pub fn level_at_end(b: &BoxF, x: f64, y: f64, r: f64, font: f64) -> bool {
    y > b.y0 - 0.25 * font && y < b.y1 + 0.25 * font && (b.x0 - x).max(x - b.x1) - r < END_CLEAR * font
}

/// True when a label box reads as belonging to its own blip (x, y, clearance r): every other
/// blip is at least `ASSOC` font sizes further from the label's attachment points, none comes
/// within `BODY_GAP` font sizes of the label, none sits in the strip between the label and its
/// blip (see `facing_zone`), so a label never spans two blips, and none sits level with the
/// label just beyond its ends (see `level_at_end`).
pub fn reads_as_own(b: &BoxF, own: (f64, f64, f64), mut others: impl Iterator<Item = (f64, f64, f64)>, font: f64) -> bool {
    let g = attach_dist(b, own.0, own.1, own.2);
    let zone = facing_zone(b, own.0, own.1, font);
    others.all(|(x, y, r)| attach_dist(b, x, y, r) >= g + ASSOC * font && box_dist(b, x, y) - r >= BODY_GAP * font && box_dist(&zone, x, y) >= r && !level_at_end(b, x, y, r, font))
}

/// Distance from a point to a segment.
pub fn seg_dist(s: &Lead, x: f64, y: f64) -> f64 {
    let (dx, dy) = (s[2] - s[0], s[3] - s[1]);
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 { 0.0 } else { (((x - s[0]) * dx + (y - s[1]) * dy) / l2).clamp(0.0, 1.0) };
    (s[0] + t * dx - x).hypot(s[1] + t * dy - y)
}

/// True when a segment touches a box (Liang-Barsky clipping).
pub fn seg_hits(s: &Lead, b: &BoxF) -> bool {
    let (dx, dy) = (s[2] - s[0], s[3] - s[1]);
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (p, q) in [(-dx, s[0] - b.x0), (dx, b.x1 - s[0]), (-dy, s[1] - b.y0), (dy, b.y1 - s[1])] {
        if p == 0.0 {
            if q < 0.0 {
                return false;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    t0 <= t1
}

/// Shortest distance between two segments.
pub fn segs_dist(a: &Lead, b: &Lead) -> f64 {
    let cross = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
    let (a0, a1, b0, b1) = ([a[0], a[1]], [a[2], a[3]], [b[0], b[1]], [b[2], b[3]]);
    if cross(a0, a1, b0).signum() * cross(a0, a1, b1).signum() < 0.0 && cross(b0, b1, a0).signum() * cross(b0, b1, a1).signum() < 0.0 {
        return 0.0;
    }
    seg_dist(a, b0[0], b0[1]).min(seg_dist(a, b1[0], b1[1])).min(seg_dist(b, a0[0], a0[1])).min(seg_dist(b, a1[0], a1[1]))
}

/// Splits a long name at the space closest to its middle.
pub fn wrap2(text: &str) -> Option<Vec<String>> {
    let n = text.chars().count();
    if n < 14 {
        return None;
    }
    let mid = n as f64 / 2.0;
    let cut = text.char_indices().filter(|&(_, c)| c == ' ').map(|(i, _)| i).min_by(|&a, &b| {
        let da = (text[..a].chars().count() as f64 - mid).abs();
        let db = (text[..b].chars().count() as f64 - mid).abs();
        da.total_cmp(&db)
    })?;
    Some(vec![text[..cut].to_string(), text[cut + 1..].to_string()])
}

/// Box of a block of lines that share x and anchor, first baseline at y.
fn block_box(x: f64, y: f64, anchor: &str, lines: &[String], font: f64) -> BoxF {
    let widest = lines.iter().max_by_key(|l| l.chars().count()).unwrap();
    let b = text_box(x, y, anchor, widest, font);
    BoxF { y1: b.y1 + (lines.len() - 1) as f64 * LINE * font, ..b }
}

/// A candidate spot for one label: its box, where the text sits, whether the name wraps onto
/// two lines, the leader line when the label sits away from its blip, and whether the label is
/// level with its blip (beside it, centred on its height, or at the end of a leader).
#[derive(Clone, Copy)]
struct Cand {
    b: BoxF,
    x: f64,
    y: f64,
    anchor: &'static str,
    wrapped: bool,
    lead: Option<Lead>,
    level: bool,
}

/// True when two placed labels get in each other's way: boxes too close, a leader through the
/// other label, or two leaders that cross or nearly touch. Labels level with their blips may
/// stack `STACK_GAP` apart, like rows of a list. A label above, below or at a corner of its blip
/// keeps `LOOSE_GAP` from any label above or below it, so two such labels never form one block
/// between two blips.
fn cands_clash(a: &Cand, b: &Cand, font: f64) -> bool {
    (if a.level && b.level { labels_clash(&a.b, &b.b, font) } else { a.b.grow(2.0 * font, LOOSE_GAP * font).hits(&b.b) })
        || a.lead.is_some_and(|s| seg_hits(&s, &b.b.grow(LEAD_GAP, LEAD_GAP)))
        || b.lead.is_some_and(|s| seg_hits(&s, &a.b.grow(LEAD_GAP, LEAD_GAP)))
        || matches!((a.lead, b.lead), (Some(p), Some(q)) if segs_dist(&p, &q) < LEAD_GAP)
}

/// Every candidate spot for one label, best first. First the spots right beside the blip:
/// outward, above or below, inward, at a corner; on one line, then on two. Then spots a little
/// away, joined to the blip by a leader, shortest leader first. `Spots::free` says which are
/// free. Returns the candidates and how many sit right beside the blip.
fn cands(it: &LabelItem, font: f64) -> (Vec<Cand>, usize) {
    let (x, y, r) = (it.x, it.y, it.r);
    let out_right = x >= 0.0;
    let s = if out_right { 1.0 } else { -1.0 };
    let step = 0.55 * LINE * font;
    let one = vec![it.text.clone()];
    let mut out = Vec::new();
    for lines in std::iter::once(one).chain(wrap2(&it.text)) {
        let n = lines.len() as f64;
        let w = lines.iter().map(|l| l.chars().count()).max().unwrap() as f64 * font * MONO_ADVANCE;
        let h = ((n - 1.0) * LINE + ASC + DESC) * font;
        let mid = y - h / 2.0 + ASC * font;
        let off = r + 0.3 * font;
        let side = |right: bool, dy: f64| if right { (x + off, mid + dy, "start") } else { (x - off, mid + dy, "end") };
        let vert = |up: bool, shift: f64| (x + shift, if up { y - r - 0.5 - h + ASC * font } else { y + r + 0.5 + ASC * font }, "middle");
        // Level with the blip first (outward, inward), then above and below, then a little
        // higher or lower, then the corners.
        let mut cs = vec![side(out_right, 0.0), side(!out_right, 0.0), vert(true, 0.0), vert(false, 0.0)];
        for dy in [-step, step] {
            cs.push(side(out_right, dy));
            cs.push(side(!out_right, dy));
        }
        for (dx, dy) in [(1.0, -1.0), (1.0, 1.0), (-1.0, -1.0), (-1.0, 1.0)] {
            let dx = dx * s;
            let ax = x + dx * r * 0.7;
            let ay = if dy < 0.0 { y - r * 0.7 - h + ASC * font } else { y + r * 0.7 + ASC * font };
            cs.push((ax, ay, if dx > 0.0 { "start" } else { "end" }));
        }
        for k in [0.3, 0.45] {
            for up in [true, false] {
                cs.push(vert(up, s * w * k));
                cs.push(vert(up, -s * w * k));
            }
        }
        for dy in [-2.0 * step, 2.0 * step] {
            cs.push(side(out_right, dy));
            cs.push(side(!out_right, dy));
        }
        for (k, (ax, ay, anchor)) in cs.into_iter().enumerate() {
            let level = k < 2;
            out.push(Cand { b: block_box(ax, ay, anchor, &lines, font), x: ax, y: ay, anchor, wrapped: lines.len() > 1, lead: None, level });
        }
    }
    let direct = out.len();
    // Leader spots: the point where the leader meets the label, then the label's anchor.
    let w = it.text.chars().count() as f64 * font * MONO_ADVANCE;
    let h = (ASC + DESC) * font;
    let mut leads: Vec<(f64, f64, &'static str)> = Vec::new();
    for d in [1.4, 2.2, 3.0, 3.8, 4.8] {
        for j in [0.0, -1.0, 1.0, -2.0, 2.0, -3.0, 3.0, -4.0, 4.0] {
            for right in [out_right, !out_right] {
                let sx = if right { 1.0 } else { -1.0 };
                leads.push((x + sx * (r + d * font), y + j * 0.75 * LINE * font, if right { "start" } else { "end" }));
            }
        }
        for up in [true, false] {
            for k in [0.0, 0.25, -0.25] {
                leads.push((x + s * k * w, y + if up { -1.0 } else { 1.0 } * (r + d * font), if up { "up" } else { "down" }));
            }
        }
    }
    leads.sort_by(|a, b| (a.0 - x).hypot(a.1 - y).total_cmp(&(b.0 - x).hypot(b.1 - y)));
    for (px, py, side) in leads {
        let (ax, ay, anchor, end) = match side {
            "start" => (px, py - h / 2.0 + ASC * font, "start", (px - LEAD_END, py)),
            "end" => (px, py - h / 2.0 + ASC * font, "end", (px + LEAD_END, py)),
            "up" => (px, py - DESC * font, "middle", (px, py + LEAD_END)),
            _ => (px, py + ASC * font, "middle", (px, py - LEAD_END)),
        };
        let (ux, uy) = (end.0 - x, end.1 - y);
        let len = ux.hypot(uy);
        if len > r + LEAD_END {
            let lead = Some([x + ux / len * r, y + uy / len * r, end.0, end.1]);
            out.push(Cand { b: text_box(ax, ay, anchor, &it.text, font), x: ax, y: ay, anchor, wrapped: false, lead, level: true });
        }
    }
    (out, direct)
}

/// The candidates of one label, checked lazily: a candidate is free when it stays inside the
/// bounds, clears every obstacle, and reads as belonging to its own blip (a leader must also
/// keep clear of other blips and obstacles).
struct Spots<'a> {
    it: &'a LabelItem,
    cs: Vec<Cand>,
    direct: usize,
    near: Vec<(f64, f64, f64)>,
    obstacles: Vec<&'a BoxF>,
    free: Vec<std::cell::Cell<u8>>,
}

impl<'a> Spots<'a> {
    fn new(it: &'a LabelItem, items: &[LabelItem], obstacles: &'a [BoxF], font: f64, bounds: &BoxF) -> Spots<'a> {
        let (cs, direct) = if it.text.is_empty() { (Vec::new(), 0) } else { cands(it, font) };
        // Only what can touch a label inside the bounds counts: the rest is culled once.
        let near = items.iter().filter(|o| o.blip != it.blip && box_dist(bounds, o.x, o.y) - o.r < (ASSOC + LEAD_CLEAR + 2.0) * font).map(|o| (o.x, o.y, o.r)).collect();
        let obstacles = obstacles.iter().filter(|o| o.hits(bounds)).collect();
        let free = vec![std::cell::Cell::new(0); cs.len()];
        Spots { it, cs, direct, near, obstacles, free }
    }

    fn free(&self, k: usize, font: f64, bounds: &BoxF) -> bool {
        let memo = self.free[k].get();
        if memo != 0 {
            return memo == 1;
        }
        let c = &self.cs[k];
        let (x, y, r) = (self.it.x, self.it.y, self.it.r);
        let ok = c.b.inside(bounds)
            && !self.obstacles.iter().any(|o| o.hits(&c.b))
            && match c.lead {
                None => reads_as_own(&c.b, (x, y, r), self.near.iter().copied(), font),
                Some(seg) => {
                    self.near.iter().all(|&(ox, oy, or)| {
                        attach_dist(&c.b, ox, oy, or) >= LEAD_CLEAR * font
                            && box_dist(&c.b, ox, oy) - or >= BODY_GAP * font
                            && seg_dist(&seg, ox, oy) >= or + LEAD_GAP
                            && !level_at_end(&c.b, ox, oy, or, font)
                    }) && !self.obstacles.iter().any(|o| box_dist(o, x, y) > 0.0 && seg_hits(&seg, o))
                }
            };
        self.free[k].set(if ok { 1 } else { 2 });
        ok
    }

    fn label(&self, k: usize) -> Label {
        let c = &self.cs[k];
        let lines = if c.wrapped { wrap2(&self.it.text).unwrap() } else { vec![self.it.text.clone()] };
        Label { blip: self.it.blip, x: c.x, y: c.y, anchor: c.anchor, lines, lead: c.lead }
    }
}

/// Greedy label placement with repair. Each label takes the first free spot (see `cands`) that
/// keeps its distance from the labels and leaders already placed. A label without a spot then
/// tries each of its spots again, moving the one label in its way to another free spot. A label
/// that still finds no spot is left out: it shows on hover and in the zoomed view. Nothing ever
/// overlaps.
///
/// Required labels (changes) always go first. Several orders run, starting with the most
/// crowded labels, which have the fewest free spots. The order that shows the most required
/// labels, then the most labels, then the fewest leaders, the fewest wrapped labels and the
/// best spots wins.
pub fn place_labels(items: &[LabelItem], obstacles: &[BoxF], font: f64, bounds: BoxF) -> Vec<Label> {
    place_labels_with(items, obstacles, font, bounds, 5 + SHUFFLES).0
}

/// `place_labels`, trying only the first `orders` label orders. Also returns the slack: the
/// free spots of the labels left out, up to 30 each.
fn place_labels_with(items: &[LabelItem], obstacles: &[BoxF], font: f64, bounds: BoxF, orders: usize) -> (Vec<Label>, usize) {
    let n = items.len();
    let sp: Vec<Spots> = items.iter().map(|it| Spots::new(it, items, obstacles, font, &bounds)).collect();
    let free = |i: usize, k: usize| sp[i].free(k, font, &bounds);
    // Free spots right beside the blip: few means crowded.
    let room: Vec<usize> = (0..n).map(|i| (0..sp[i].direct).filter(|&k| free(i, k)).count()).collect();
    let base: Vec<usize> = (0..n).filter(|&i| !items[i].text.is_empty()).collect();
    let len = |i: &usize| items[*i].text.chars().count();
    let dist = |i: &usize| (items[*i].x.hypot(items[*i].y) * 10.0) as i64;
    let crowd = |i: &usize| items.iter().filter(|o| (o.x - items[*i].x).hypot(o.y - items[*i].y) < 70.0).count();
    let req = |i: &usize| !items[*i].required;
    let orders: Vec<Vec<usize>> = (0..orders)
        .map(|key| {
            let mut o = base.clone();
            match key {
                0 => o.sort_by_key(|i| (req(i), std::cmp::Reverse(crowd(i)), room[*i])),
                1 => o.sort_by_key(|i| (req(i), std::cmp::Reverse(len(i)))),
                2 => o.sort_by_key(|i| (req(i), room[*i])),
                3 => o.sort_by_key(|i| (req(i), dist(i))),
                4 => o.sort_by_key(|i| (req(i), std::cmp::Reverse(dist(i)))),
                _ => o.sort_by_key(|i| (req(i), (items[*i].blip as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15 ^ key as u64).rotate_left(17))),
            }
            o
        })
        .collect();
    let fits = |i: usize, k: usize, pick: &[Option<usize>]| free(i, k) && base.iter().all(|&j| j == i || pick[j].is_none_or(|q| !cands_clash(&sp[i].cs[k], &sp[j].cs[q], font)));
    let first = |i: usize, pick: &[Option<usize>]| (0..sp[i].cs.len()).find(|&k| fits(i, k, pick));
    let score = |pick: &[Option<usize>]| {
        let on: Vec<(usize, usize)> = pick.iter().enumerate().filter_map(|(i, p)| p.map(|k| (i, k))).collect();
        let reqd = on.iter().filter(|(i, _)| items[*i].required).count();
        let leads = on.iter().filter(|(i, k)| sp[*i].cs[*k].lead.is_some()).count();
        let wrapped = on.iter().filter(|(i, k)| sp[*i].cs[*k].wrapped).count();
        let rank: usize = on.iter().map(|(_, k)| k).sum();
        (reqd, on.len(), std::cmp::Reverse(leads), std::cmp::Reverse(wrapped), std::cmp::Reverse(rank))
    };
    let mut best: Option<Vec<Option<usize>>> = None;
    for o in &orders {
        let mut pick: Vec<Option<usize>> = vec![None; n];
        for &i in o {
            pick[i] = first(i, &pick);
        }
        // Repair: a label without a spot moves one label out of its way.
        loop {
            let mut moved = false;
            for &u in o {
                if pick[u].is_some() {
                    continue;
                }
                'spot: for k in 0..sp[u].cs.len() {
                    if !free(u, k) {
                        continue;
                    }
                    let mut blocking = base.iter().copied().filter(|&j| j != u && pick[j].is_some_and(|q| cands_clash(&sp[u].cs[k], &sp[j].cs[q], font)));
                    let (Some(j), None) = (blocking.next(), blocking.next()) else { continue };
                    let old = pick[j];
                    pick[j] = None;
                    pick[u] = Some(k);
                    if let Some(t) = (0..sp[j].cs.len()).find(|&t| Some(t) != old && fits(j, t, &pick)) {
                        pick[j] = Some(t);
                        moved = true;
                        break 'spot;
                    }
                    pick[u] = None;
                    pick[j] = old;
                }
                if pick[u].is_none() {
                    pick[u] = first(u, &pick);
                    moved |= pick[u].is_some();
                }
            }
            if !moved {
                break;
            }
        }
        // Tidy: each label takes the best spot that is free now.
        for &i in o {
            if let Some(k) = pick[i]
                && let Some(b) = (0..k).find(|&b| fits(i, b, &pick))
            {
                pick[i] = Some(b);
            }
        }
        let done = base.iter().all(|&i| pick[i].is_some_and(|k| sp[i].cs[k].lead.is_none() && !sp[i].cs[k].wrapped));
        if best.as_ref().is_none_or(|b| score(&pick) > score(b)) {
            best = Some(pick);
        }
        if done {
            break;
        }
    }
    let best = best.unwrap_or_else(|| vec![None; n]);
    let slack = base.iter().filter(|&&i| best[i].is_none()).map(|&i| (0..sp[i].cs.len()).filter(|&k| free(i, k)).count().min(30)).sum();
    (best.iter().enumerate().filter_map(|(i, k)| k.map(|k| sp[i].label(k))).collect(), slack)
}
