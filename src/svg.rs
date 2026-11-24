//! Inline SVG for the browser: the radar scope, the quadrant close-up and the ring track.

use crate::html::{Ctx, esc};
use crate::layout::{self, BLIP_R, BoxF, F_OVER, F_ZOOM, F_ZOOM_M, Label, R, R_DOT, R_HALO, RING_OUTER, VB, Z_BLIP, overview_bounds, zoom_bounds};
use crate::model::Corner;
use crate::view::{RadarView, State};
use std::f64::consts::PI;
use std::fmt::Write;

/// Ring names on the axis (site.css `.rlab`): 10px, 6px when zoomed, 0.65em advance at 87.5%
/// width plus 0.14em letter spacing.
const RLAB_ADV: f64 = 0.79;

/// Fixed scope marks that labels must avoid: ring names on the axis and, in the overview,
/// the axis lines.
pub fn furniture_boxes(ctx: &Ctx, zoomed: bool) -> Vec<BoxF> {
    let mut v = Vec::new();
    let fs = if zoomed { 6.0 } else { 10.0 };
    for (k, r) in ctx.m.site.rings.iter().enumerate() {
        let (a, b) = layout::band(k);
        let w = r.name.chars().count() as f64 * fs * RLAB_ADV;
        for x in [(a + b) / 2.0, -(a + b) / 2.0] {
            v.push(BoxF { x0: x - w / 2.0, y0: -6.0 - layout::ASC * fs, x1: x + w / 2.0, y1: -6.0 + layout::DESC * fs }.grow(1.5, 1.0));
        }
    }
    if !zoomed {
        v.extend(layout::axis_lines(F_OVER));
    }
    v
}

fn f(v: f64) -> String {
    let s = format!("{:.1}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

fn corners(ctx: &Ctx) -> Vec<Corner> {
    ctx.m.site.quadrants.iter().map(|q| q.position).collect()
}

/// Static scope furniture: bands, bearing ticks, axes, ring and quadrant labels. The ticks
/// carry no numbers: on this radar a bearing means nothing.
fn furniture(ctx: &Ctx, o: &mut String, quad_labels: bool) {
    o.push_str("<g class=\"bands\">");
    for k in (0..4).rev() {
        let _ = write!(o, "<circle class=\"band b{k}\" r=\"{}\"/>", RING_OUTER[k]);
    }
    o.push_str("</g>");
    // Bearing ticks every 5°, longer every 30°, like a plan position indicator.
    let mut d = String::new();
    for i in 0..72 {
        let a = i as f64 * 5.0 * PI / 180.0;
        let len = if i % 6 == 0 {
            12.0
        } else if i % 2 == 0 {
            7.0
        } else {
            4.0
        };
        let (s, c) = a.sin_cos();
        let _ = write!(d, "M{} {}L{} {}", f(c * R), f(s * R), f(c * (R + len)), f(s * (R + len)));
    }
    let _ = write!(o, "<path class=\"ticks\" d=\"{d}\"/>");
    let _ = write!(o, "<path class=\"axes\" d=\"M{a} 0H{b}M0 {a}V{b}\"/>", a = -R, b = R);
    o.push_str("<g class=\"rlab\">");
    for (k, r) in ctx.m.site.rings.iter().enumerate() {
        let (a, b) = layout::band(k);
        let x = (a + b) / 2.0;
        let name = esc(&r.name.to_uppercase());
        let _ = write!(o, "<text x=\"{}\" y=\"-6\">{name}</text><text x=\"{}\" y=\"-6\">{name}</text>", f(x), f(-x));
    }
    o.push_str("</g>");
    if quad_labels {
        for (q, quad) in ctx.m.site.quadrants.iter().enumerate() {
            let (sx, sy) = layout::corner_sign(quad.position);
            let anchor = if sx < 0.0 { "start" } else { "end" };
            let x = sx * (VB - 6.0);
            let words: Vec<String> = quad.name.to_uppercase().split(' ').map(String::from).collect();
            // Join "&" onto the previous word; one word per line otherwise.
            let mut lines: Vec<String> = Vec::new();
            for w in words {
                if w == "&"
                    && let Some(l) = lines.last_mut()
                {
                    l.push_str(" &");
                    continue;
                }
                lines.push(w);
            }
            let lh = 19.0;
            let y0 = if sy < 0.0 { -VB + 20.0 } else { VB - 8.0 - lh * (lines.len() as f64 - 1.0) };
            let _ = write!(o, "<g class=\"qlab\" data-q=\"{q}\"><text text-anchor=\"{anchor}\">");
            for (li, l) in lines.iter().enumerate() {
                let _ = write!(o, "<tspan x=\"{}\" y=\"{}\">{}</tspan>", f(x), f(y0 + li as f64 * lh), esc(l));
            }
            o.push_str("</text></g>");
        }
    }
}

/// Label font size for a label group class (site.css `.lbl`).
fn labels_font(class: &str) -> f64 {
    if !class.starts_with("lz") {
        F_OVER
    } else if class.ends_with(" m") {
        F_ZOOM_M
    } else {
        F_ZOOM
    }
}

/// The band under the middle of a label (0 to 3, or 4 outside the scope). The label's halo
/// takes that band's colour, so it never shows as a tint of another band.
fn halo_band(l: &Label, font: f64) -> usize {
    let w = l.lines.iter().map(|s| s.chars().count()).max().unwrap_or(0) as f64 * font * layout::MONO_ADVANCE;
    let cx = match l.anchor {
        "end" => l.x - w / 2.0,
        "middle" => l.x,
        _ => l.x + w / 2.0,
    };
    let h = ((l.lines.len() as f64 - 1.0) * layout::LINE + layout::ASC + layout::DESC) * font;
    let cy = l.y - layout::ASC * font + h / 2.0;
    RING_OUTER.iter().position(|&o| cx.hypot(cy) <= o).unwrap_or(4)
}

fn labels_group(o: &mut String, class: &str, labels: &[Label]) {
    let _ = write!(o, "<g class=\"lbl {class}\">");
    // Leader lines first, so no line ever runs over a label.
    for l in labels {
        if let Some([x0, y0, x1, y1]) = l.lead {
            let _ = write!(o, "<path class=\"lead\" data-i=\"{}\" d=\"M{} {}L{} {}\"/>", l.blip, f(x0), f(y0), f(x1), f(y1));
        }
    }
    let font = labels_font(class);
    for l in labels {
        let _ = write!(o, "<text class=\"h{}\" x=\"{}\" y=\"{}\" text-anchor=\"{}\" data-i=\"{}\">", halo_band(l, font), f(l.x), f(l.y), l.anchor, l.blip);
        if let [one] = &l.lines[..] {
            o.push_str(&esc(one));
        } else {
            // Wrapped name: one tspan per line, the em boxes stacked.
            for (k, line) in l.lines.iter().enumerate() {
                let _ = write!(o, "<tspan x=\"{}\" y=\"{}\">{}</tspan>", f(l.x), f(l.y + k as f64 * layout::LINE * font), esc(line));
            }
        }
        o.push_str("</text>");
    }
    o.push_str("</g>");
}

/// The state each blip shows at the latest edition. The curated radar colours movement; any
/// other radar colours disagreement with it.
fn states(ctx: &Ctx, rv: &RadarView) -> Vec<State> {
    let e = rv.latest();
    let main = ctx.m.main();
    rv.blips
        .iter()
        .map(|b| {
            if std::ptr::eq(rv, main) {
                return b.state_at(e);
            }
            match (b.rings[e], main.find(&b.e.id).and_then(|m| m.ring_now())) {
                (None, _) => State::Absent,
                (Some(_), None) => State::New,
                (Some(k), Some(o)) if k < o => State::In,
                (Some(k), Some(o)) if k > o => State::Out,
                _ => State::Steady,
            }
        })
        .collect()
}

/// Clearance radius of each blip: the halo for new blips, else the dot.
fn blip_r(states: &[State], i: usize, zoom: f64) -> f64 {
    zoom * if states[i] == State::New { R_HALO } else { R_DOT }
}

fn label_items(rv: &RadarView, pos: &[Option<layout::Pos>], states: &[State]) -> Vec<layout::LabelItem> {
    pos.iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let p = p.as_ref()?;
            let required = matches!(states[i], State::New | State::In | State::Out);
            Some(layout::LabelItem { blip: i, x: p.x, y: p.y, r: blip_r(states, i, 1.0), text: layout::scope_text(rv.blips[i].e), required })
        })
        .collect()
}

/// The overview's labels at the latest edition: one per blip where it fits (on Konstantin's
/// radar, every blip). The builder reuses them for blips that sit where they sit here.
pub fn overview_labels(ctx: &Ctx, rv: &RadarView) -> Vec<Label> {
    let cs = corners(ctx);
    let pos = layout::layout(rv, &cs, rv.latest());
    let states = states(ctx, rv);
    let mut obstacles: Vec<BoxF> = furniture_boxes(ctx, false);
    obstacles.extend(pos.iter().enumerate().filter_map(|(i, p)| p.map(|p| BoxF::around(p.x, p.y, blip_r(&states, i, 1.0)))));
    obstacles.extend(layout::trail_obstacles(rv, &pos));
    let items = label_items(rv, &pos, &states);
    // The axes keep overview labels inside their quadrant, so each quadrant is placed on its own.
    let mut placed: Vec<Label> = Vec::new();
    for (q, &c) in cs.iter().enumerate() {
        let qitems: Vec<layout::LabelItem> =
            items.iter().map(|it| layout::LabelItem { text: if rv.blips[it.blip].q == q { it.text.clone() } else { String::new() }, ..*it }).collect();
        placed.extend(layout::place_labels(&qitems, &obstacles, F_OVER, overview_bounds(c)));
    }
    placed
}

/// The full radar scope. `interactive` adds the data the client script needs.
pub fn scope(ctx: &Ctx, rv: &RadarView, from: &str, interactive: bool) -> String {
    let e = rv.latest();
    let cs = corners(ctx);
    let pos = layout::layout(rv, &cs, e);
    let states = states(ctx, rv);
    let mut o = String::with_capacity(48 * 1024);
    let _ = write!(o, "<svg class=\"radar\" viewBox=\"{a} {a} {w} {w}\" aria-hidden=\"true\" focusable=\"false\"><g class=\"z\">", a = -VB, w = 2.0 * VB);
    furniture(ctx, &mut o, true);

    // Trails: where recently moved blips came from.
    o.push_str("<g class=\"trails\">");
    for (i, p) in pos.iter().enumerate() {
        let Some(p) = p else { continue };
        let t = layout::trail(rv, i, *p, 3);
        if t.is_empty() {
            continue;
        }
        let st = states[i];
        let cls = if matches!(st, State::In | State::Out) { st.class() } else { "s-past" };
        let age = e - rv.blips[i].last_change();
        let pts: Vec<String> = t.iter().map(|(x, y, _)| format!("{},{}", f(*x), f(*y))).collect();
        let _ = write!(o, "<g class=\"trail {cls}\" data-i=\"{i}\" style=\"opacity:{}\"><polyline points=\"{}\"/>", f(1.0 - age as f64 * 0.22), pts.join(" "));
        for (x, y, _) in &t[..t.len() - 1] {
            let _ = write!(o, "<circle r=\"4\" cx=\"{}\" cy=\"{}\"/>", f(*x), f(*y));
        }
        o.push_str("</g>");
    }
    o.push_str("</g><g class=\"blips\">");
    for (i, p) in pos.iter().enumerate() {
        let Some(p) = p else { continue };
        let b = &rv.blips[i];
        let st = states[i];
        let _ = write!(
            o,
            "<a class=\"blip {} q{}\" href=\"{}\" tabindex=\"-1\" data-i=\"{i}\" style=\"transform:translate({}px,{}px)\"><circle class=\"halo\" r=\"13\"/><circle class=\"dot\" r=\"{BLIP_R}\"/></a>",
            st.class(),
            b.q,
            ctx.href(from, &format!("e/{}/", b.e.id)),
            f(p.x),
            f(p.y),
        );
    }
    o.push_str("</g>");

    // Labels: changes first; `place_labels` picks the order that fits the most.
    let items = label_items(rv, &pos, &states);
    let placed = overview_labels(ctx, rv);
    let (s_lbl, r_lbl): (Vec<Label>, Vec<Label>) = placed.into_iter().partition(|l| items.iter().any(|it| it.blip == l.blip && it.required));
    labels_group(&mut o, "lo lo-sig", &s_lbl);
    labels_group(&mut o, "lo lo-rest", &r_lbl);

    if interactive {
        let zfurn = furniture_boxes(ctx, true);
        let trail_obs = layout::trail_obstacles(rv, &pos);
        for (q, &c) in cs.iter().enumerate() {
            let qitems: Vec<layout::LabelItem> =
                items.iter().filter(|it| rv.blips[it.blip].q == q).map(|it| layout::LabelItem { text: it.text.clone(), r: blip_r(&states, it.blip, Z_BLIP), ..*it }).collect();
            let mut obs = zfurn.clone();
            obs.extend(pos.iter().enumerate().filter_map(|(i, p)| p.map(|p| BoxF::around(p.x, p.y, blip_r(&states, i, Z_BLIP)))));
            obs.extend_from_slice(&trail_obs);
            for (cls, font) in [("d", F_ZOOM), ("m", F_ZOOM_M)] {
                let l = layout::place_labels(&qitems, &obs, font, zoom_bounds(c, font));
                labels_group(&mut o, &format!("lz lz{q} {cls}"), &l);
            }
        }
    }
    o.push_str("</g></svg>");
    o
}

/// Close-up of one quadrant with a single blip in focus (entry pages).
pub fn quadrant(ctx: &Ctx, rv: &RadarView, focus: usize) -> String {
    let b = &rv.blips[focus];
    let cs = corners(ctx);
    // Show the focus at the last edition where it was on the radar.
    let e = (0..rv.dates.len()).rev().find(|&e| b.rings[e].is_some()).unwrap_or(0);
    let pos = layout::layout(rv, &cs, e);
    let (sx, sy) = layout::corner_sign(cs[b.q]);
    let m = 16.0;
    let (x0, y0) = (if sx < 0.0 { -R - m } else { -m }, if sy < 0.0 { -R - m } else { -m });
    let mut o = String::new();
    let _ = write!(o, "<svg class=\"radar quad\" viewBox=\"{} {} {} {}\" aria-hidden=\"true\" focusable=\"false\"><g>", f(x0), f(y0), f(R + 2.0 * m), f(R + 2.0 * m));
    furniture(ctx, &mut o, false);
    let fp = pos[focus].unwrap();
    // Full movement history for the focus, at its current angle.
    let mut pts = Vec::new();
    let mut prev = None;
    for k in b.rings.iter().flatten() {
        if prev != Some(*k) {
            pts.push(layout::point(fp.theta, *k, fp.frac));
            prev = Some(*k);
        }
    }
    // Edition 01 is the baseline: what it placed is not "new".
    let st = match b.state_at(rv.latest()) {
        State::New if rv.latest() == 0 => State::Steady,
        st => st,
    };
    let retired = b.ring_now().is_none();
    if pts.len() > 1 {
        let cls = if matches!(st, State::In | State::Out) { st.class() } else { "s-past" };
        let s: Vec<String> = pts.iter().map(|(x, y)| format!("{},{}", f(*x), f(*y))).collect();
        let _ = write!(o, "<g class=\"trail {cls}\"><polyline points=\"{}\"/>", s.join(" "));
        for (x, y) in &pts[..pts.len() - 1] {
            let _ = write!(o, "<circle r=\"4\" cx=\"{}\" cy=\"{}\"/>", f(*x), f(*y));
        }
        o.push_str("</g>");
    }
    o.push_str("<g class=\"blips\">");
    for (i, p) in pos.iter().enumerate() {
        let Some(p) = p else { continue };
        if rv.blips[i].q != b.q || i == focus {
            continue;
        }
        let _ = write!(o, "<circle class=\"dim\" r=\"6\" cx=\"{}\" cy=\"{}\"/>", f(p.x), f(p.y));
    }
    let cls = if retired { "s-retired" } else { st.class() };
    let _ = write!(o, "<g class=\"blip focus {cls}\" transform=\"translate({} {})\">", f(fp.x), f(fp.y));
    if st == State::New {
        o.push_str("<circle class=\"halo\" r=\"15\"/>");
    }
    // Target-lock brackets.
    o.push_str("<path class=\"lock\" d=\"M-19 -11V-19H-11M11 -19H19V-11M19 11V19H11M-11 19H-19V11\"/>");
    let _ = write!(o, "<circle class=\"dot\" r=\"{}\"/></g></g>", BLIP_R + 2.0);
    o.push_str("</g></svg>");
    o
}

/// Ring over time, one lane per ring, like an altitude profile.
pub fn track(ctx: &Ctx, rv: &RadarView, i: usize) -> String {
    let b = &rv.blips[i];
    let n = rv.dates.len();
    let (w, left, right, top, lane) = (720.0, 92.0, 16.0, 14.0, 30.0);
    let h = top + lane * 4.0 + 30.0;
    let x = |e: usize| if n == 1 { (left + w - right) / 2.0 } else { left + (w - left - right) * e as f64 / (n - 1) as f64 };
    let y = |k: usize| top + lane * (k as f64 + 0.5);
    let mut o = String::new();
    let _ = write!(o, "<svg class=\"track\" viewBox=\"0 0 {w} {h}\" role=\"img\" aria-label=\"Ring history for {}\">", esc(&b.e.name));
    for (k, r) in ctx.m.site.rings.iter().enumerate() {
        let _ = write!(
            o,
            "<path class=\"lane\" d=\"M{left} {}H{}\"/><text class=\"lane-l\" x=\"0\" y=\"{}\">{}</text>",
            f(y(k)),
            w - right,
            f(y(k) + 4.0),
            esc(&r.name.to_uppercase())
        );
    }
    for e in 0..n {
        let d = rv.dates[e];
        if n <= 14 || e % 2 == (n - 1) % 2 {
            let _ = write!(o, "<text class=\"wk\" x=\"{}\" y=\"{}\">{}</text>", f(x(e)), f(h - 8.0), d.short());
        }
    }
    // Step path; gaps where the blip is off the radar.
    let mut d = String::new();
    let mut prev: Option<usize> = None;
    for e in 0..n {
        match (prev, b.rings[e]) {
            (None, Some(k)) => {
                let _ = write!(d, "M{} {}", f(x(e)), f(y(k)));
            }
            (Some(p), Some(k)) => {
                let _ = write!(d, "H{}", f(x(e)));
                if p != k {
                    let _ = write!(d, "V{}", f(y(k)));
                }
            }
            _ => {}
        }
        prev = b.rings[e];
    }
    let _ = write!(o, "<path class=\"step\" d=\"{d}\"/>");
    for e in 0..n {
        let st = b.state_at(e);
        let k = match (st, b.rings[e]) {
            (_, Some(k)) => k,
            (State::Retired, None) => b.rings[e - 1].unwrap(),
            _ => continue,
        };
        if matches!(st, State::Steady) && e != n - 1 {
            continue;
        }
        let r = if e == n - 1 { 6.5 } else { 5.0 };
        // Edition 01 is the baseline: its entries wear no "new" colour.
        let st = if e == 0 && st == State::New { State::Steady } else { st };
        let _ = write!(o, "<circle class=\"pt {}\" cx=\"{}\" cy=\"{}\" r=\"{r}\"><title>{}: {}</title></circle>", st.class(), f(x(e)), f(y(k)), rv.dates[e].human(), st.label());
    }
    o.push_str("</svg>");
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::{Ctx, Links};
    use crate::layout::{labels_clash, text_box};
    use crate::model::Data;
    use crate::view::Model;

    fn attr<'a>(tag: &'a str, name: &str) -> &'a str {
        let k = format!(" {name}=\"");
        let i = tag.find(&k).unwrap_or_else(|| panic!("no {name} in {tag}")) + k.len();
        &tag[i..i + tag[i..].find('"').unwrap()]
    }

    /// A drawn label: the boxes of its lines, its text, its blip and its leader line.
    #[derive(Clone)]
    struct Drawn {
        boxes: Vec<BoxF>,
        text: String,
        blip: usize,
        lead: Option<layout::Lead>,
    }

    impl Drawn {
        fn block(&self) -> BoxF {
            self.boxes.iter().skip(1).fold(self.boxes[0], |a, b| BoxF { x0: a.x0.min(b.x0), y0: a.y0.min(b.y0), x1: a.x1.max(b.x1), y1: a.y1.max(b.y1) })
        }
    }

    /// Label groups in a rendered scope: (group class, font, labels), each label as the boxes
    /// of its lines measured with the renderer's own text metric.
    fn label_groups(svg: &str) -> Vec<(String, f64, Vec<Drawn>)> {
        let mut out = Vec::new();
        for part in svg.split("<g class=\"lbl ").skip(1) {
            let class = part[..part.find('"').unwrap()].to_string();
            let font = labels_font(&class);
            let body = &part[..part.find("</g>").unwrap()];
            let mut leads = std::collections::HashMap::new();
            for p in body.split("<path class=\"lead\" ").skip(1) {
                let tag = format!(" {}", &p[..p.find("/>").unwrap()]);
                let v: Vec<f64> = attr(&tag, "d").replace(['M', 'L'], " ").split_whitespace().map(|v| v.parse().unwrap()).collect();
                leads.insert(attr(&tag, "data-i").parse::<usize>().unwrap(), [v[0], v[1], v[2], v[3]]);
            }
            let mut ls = Vec::new();
            for t in body.split("<text ").skip(1) {
                let t = &t[..t.find("</text>").unwrap()];
                let tag = format!(" {}", &t[..t.find('>').unwrap()]);
                let anchor = attr(&tag, "text-anchor").to_string();
                let blip: usize = attr(&tag, "data-i").parse().unwrap();
                let inner = &t[t.find('>').unwrap() + 1..];
                // Plain text, or one tspan per line.
                let lines: Vec<(String, String, String)> = if inner.contains("<tspan") {
                    inner
                        .split("<tspan ")
                        .skip(1)
                        .map(|sp| {
                            let tg = format!(" {}", &sp[..sp.find('>').unwrap()]);
                            (attr(&tg, "x").to_string(), attr(&tg, "y").to_string(), sp[sp.find('>').unwrap() + 1..sp.find("</tspan>").unwrap()].to_string())
                        })
                        .collect()
                } else {
                    vec![(attr(&tag, "x").to_string(), attr(&tag, "y").to_string(), inner.to_string())]
                };
                let boxes = lines.iter().map(|(x, y, tx)| text_box(x.parse().unwrap(), y.parse().unwrap(), &anchor, &tx.replace("&amp;", "&"), font)).collect();
                ls.push(Drawn { boxes, text: lines.iter().map(|l| l.2.clone()).collect::<Vec<_>>().join(" "), blip, lead: leads.get(&blip).copied() });
            }
            out.push((class, font, ls));
        }
        out
    }

    /// Every blip: (index, x, y, clearance radius as placed: the halo for new blips, else the dot).
    fn blips(svg: &str) -> Vec<(usize, f64, f64, f64)> {
        svg.split("<a class=\"blip ")
            .skip(1)
            .map(|p| {
                let tag = format!(" {}", &p[..p.find('>').unwrap()]);
                let tr = &p[p.find("translate(").unwrap() + 10..];
                let mut it = tr[..tr.find(')').unwrap()].split(',').map(|v| v.trim_end_matches("px").parse::<f64>().unwrap());
                let r = if p.starts_with("s-new") { R_HALO } else { R_DOT };
                (attr(&tag, "data-i").parse().unwrap(), it.next().unwrap(), it.next().unwrap(), r)
            })
            .collect()
    }

    /// Checks one visible set of labels against the visible blips (radii scaled by `scale`);
    /// returns the problems found. `edge` gives the area each label must stay in.
    fn check(name: &str, ls: &[Drawn], font: f64, blips: &[(usize, f64, f64, f64)], scale: f64, edge: impl Fn(&Drawn) -> BoxF) -> Vec<String> {
        let mut bad = Vec::new();
        for (i, l) in ls.iter().enumerate() {
            let ta = &l.text;
            for a in &l.boxes {
                for o in &ls[i + 1..] {
                    if o.boxes.iter().any(|b| a.hits(b) || labels_clash(a, b, font)) {
                        bad.push(format!("{name}: label {ta} touches label {}", o.text));
                    }
                }
                for &(_, x, y, r) in blips {
                    // The drawn dot, without the one unit of clearance.
                    if a.hits(&BoxF::around(x, y, (r - 1.0) * scale)) {
                        bad.push(format!("{name}: label {ta} covers blip at {x},{y}"));
                    }
                }
                if !a.inside(&edge(l)) {
                    bad.push(format!("{name}: label {ta} leaves its area"));
                }
            }
            let Some(&(_, ox, oy, or)) = blips.iter().find(|b| b.0 == l.blip) else {
                bad.push(format!("{name}: label {ta} has no visible blip"));
                continue;
            };
            let others: Vec<(f64, f64, f64)> = blips.iter().filter(|b| b.0 != l.blip).map(|&(_, x, y, r)| (x, y, r * scale)).collect();
            let block = l.block();
            // No other dot sits level with the label just past its ends ("Guard hooks ●"), with
            // or without a leader.
            for &(x, y, r) in &others {
                if layout::level_at_end(&block, x, y, r, font) {
                    bad.push(format!("{name}: label {ta} sits level with another blip at {x},{y}"));
                }
            }
            match l.lead {
                None => {
                    if !layout::reads_as_own(&block, (ox, oy, or * scale), others.iter().copied(), font) {
                        bad.push(format!("{name}: label {ta} does not read as its own blip's"));
                    }
                    // Without a leader, the label's nearest dot is its own.
                    let own = layout::box_dist(&block, ox, oy) - or * scale;
                    if others.iter().any(|&(x, y, r)| layout::box_dist(&block, x, y) - r <= own) {
                        bad.push(format!("{name}: label {ta} is nearer another blip than its own"));
                    }
                }
                Some(s) => {
                    // A leader keeps clear of every other blip's clearance circle by LEAD_GAP
                    // (0.1 of slack for the one-decimal coordinates in the SVG).
                    if others.iter().any(|&(x, y, r)| layout::seg_dist(&s, x, y) < r + layout::LEAD_GAP - 0.1) {
                        bad.push(format!("{name}: leader of {ta} runs through or past another blip"));
                    }
                    if ls.iter().any(|o| o.blip != l.blip && o.boxes.iter().any(|b| layout::seg_hits(&s, b))) {
                        bad.push(format!("{name}: leader of {ta} runs through another label"));
                    }
                }
            }
        }
        bad
    }

    #[test]
    fn real_scopes_have_no_label_collisions() {
        let d = Data::load(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/data"))).expect("data parses");
        let m = Model::new(&d);
        let ctx = Ctx { m: &m, links: Links::Dirs, css: String::new(), base_url: String::new(), versions: vec![] };
        let cs = corners(&ctx);
        let quad = |q: usize| {
            let (sx, sy) = layout::corner_sign(cs[q]);
            BoxF { x0: sx.min(0.0) * VB, y0: sy.min(0.0) * VB, x1: sx.max(0.0) * VB, y1: sy.max(0.0) * VB }
        };
        let mut bad = Vec::new();
        let mut shown = Vec::new();
        for (ri, rv) in m.radars.iter().enumerate() {
            let svg = scope(&ctx, rv, "", true);
            let bl = blips(&svg);
            let groups = label_groups(&svg);
            // Overview: both `lo` groups show together, each label inside its blip's quadrant.
            let over: Vec<Drawn> = groups.iter().filter(|g| g.0.starts_with("lo")).flat_map(|g| g.2.clone()).collect();
            shown.push(format!("radar {ri}: {} of {} labels in the overview", over.len(), bl.len()));
            bad.extend(check(&format!("radar {ri} overview"), &over, F_OVER, &bl, 1.0, |l| quad(rv.blips[l.blip].q)));
            // Konstantin's radar names every blip in the overview.
            if ri == 0 && over.len() != bl.len() {
                let missing: Vec<&str> = bl.iter().filter(|b| !over.iter().any(|l| l.blip == b.0)).map(|b| rv.blips[b.0].e.name.as_str()).collect();
                bad.push(format!("radar 0 overview: {} of {} blips have no label: {}", missing.len(), bl.len(), missing.join(", ")));
            }
            // Zoomed quadrants: one `lz` group at a time, only that quadrant's blips, drawn at 72%.
            for (class, font, ls) in groups.iter().filter(|g| g.0.starts_with("lz")) {
                let q: usize = class[5..6].parse().unwrap();
                let qb: Vec<(usize, f64, f64, f64)> = bl.iter().copied().filter(|b| rv.blips[b.0].q == q).collect();
                shown.push(format!("radar {ri} {class}: {} of {}", ls.len(), qb.len()));
                bad.extend(check(&format!("radar {ri} {class}"), ls, *font, &qb, Z_BLIP, |_| quad(q)));
            }
            // The entry close-ups and the replay draw no label text; keep it that way.
            for i in (0..rv.blips.len()).filter(|_| ri == 0) {
                assert!(!quadrant(&ctx, rv, i).contains("class=\"lbl"), "entry close-up gained labels: revisit this test");
            }
        }
        eprintln!("{}", shown.join("\n"));
        assert!(bad.is_empty(), "{} label problems:\n{}", bad.len(), bad.join("\n"));
    }
}
