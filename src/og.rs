//! Social cards (1200x630 PNG via resvg), README badges, favicon and touch icon.
//!
//! Cards are the unit people share, so each entry, edition and radar gets its own.

use crate::html::{Ctx, esc};
use crate::layout::{self, R};
use crate::view::{RadarView, State};
use resvg::{tiny_skia, usvg};
use std::fmt::Write;

// Family names as fontdb reads them from the static TTFs (see `radar fonts`).
// Dark "scope" palette for cards: stands out in light and dark feeds alike.
// The dark tokens of site.css: ground, ink, rules, and the teal ring ramp (marks and fills).
const BG: &str = "#0f171d";
const INK: &str = "#e1e7eb";
const INK2: &str = "#9aabb7";
const INK3: &str = "#58656f";
const RULE: &str = "#343f47";
const BANDS: [&str; 4] = ["#14362e", "#162c26", "#172320", "#161d1b"];
const RINGS: [&str; 4] = ["#5fb6a1", "#5a9e8d", "#54887a", "#4d7268"];
const RING_BG: [&str; 4] = ["#133e34", "#18322c", "#1a2925", "#19211f"];
const C_NEW: &str = "#e0468f";
const C_IN: &str = "#4a86f0";
const C_OUT: &str = "#e06f22";

const DISPLAY: &str = "font-family=\"Archivo Expanded\" font-weight=\"800\" font-stretch=\"expanded\"";
const TEXT: &str = "font-family=\"Archivo\" font-weight=\"500\"";
const MONO: &str = "font-family=\"Martian Mono SemiCondensed\" font-weight=\"500\" font-stretch=\"semi-condensed\"";

/// Advance widths (per 1000 em) for ASCII 32..=126, from the TTFs in assets/og-fonts.
const W_DISPLAY: [u16; 95] = [
    304, 358, 522, 757, 710, 1132, 1001, 278, 371, 371, 431, 736, 349, 416, 349, 308, 786, 703, 783, 789, 781, 789, 791, 726, 803, 791, 344, 349, 736, 736, 736, 715, 1253, 933,
    914, 942, 933, 863, 799, 1012, 989, 373, 734, 951, 754, 1148, 988, 1013, 861, 1013, 932, 867, 848, 968, 904, 1213, 940, 918, 857, 360, 308, 360, 736, 655, 319, 776, 765, 771,
    765, 780, 482, 766, 755, 315, 313, 733, 315, 1166, 755, 781, 765, 765, 480, 710, 501, 755, 706, 1078, 764, 706, 660, 363, 293, 363, 736,
];
const W_TEXT: [u16; 95] = [
    205, 282, 407, 582, 524, 958, 710, 227, 356, 356, 407, 630, 288, 333, 288, 296, 574, 547, 571, 575, 566, 573, 575, 564, 575, 574, 315, 315, 630, 630, 630, 595, 1002, 695, 702,
    725, 731, 674, 611, 795, 734, 274, 571, 677, 552, 846, 734, 785, 667, 785, 722, 670, 612, 728, 659, 938, 683, 666, 635, 316, 296, 316, 630, 495, 197, 550, 579, 532, 579, 554,
    293, 573, 573, 238, 236, 528, 238, 860, 573, 583, 579, 579, 346, 524, 305, 572, 516, 740, 529, 516, 503, 372, 245, 372, 630,
];

fn width(table: &[u16; 95], s: &str, size: f64) -> f64 {
    s.chars().map(|c| if (' '..='~').contains(&c) { table[c as usize - 32] as f64 } else { 800.0 }).sum::<f64>() * size / 1000.0
}

fn mono_w(s: &str, size: f64) -> f64 {
    s.chars().count() as f64 * size * layout::MONO_ADVANCE
}

/// Greedy word wrap.
fn wrap(table: &[u16; 95], text: &str, size: f64, max_w: f64) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(l) if width(table, &format!("{l} {word}"), size) <= max_w => {
                l.push(' ');
                l.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

/// Largest size (from `sizes`, descending) at which the text wraps into `max_lines` lines within `max_w`.
fn fit(table: &[u16; 95], text: &str, sizes: &[f64], max_w: f64, max_lines: usize) -> (f64, Vec<String>) {
    for &s in sizes {
        let l = wrap(table, text, s, max_w);
        if l.len() <= max_lines && l.iter().all(|x| width(table, x, s) <= max_w) {
            return (s, l);
        }
    }
    let s = *sizes.last().unwrap();
    (s, wrap(table, text, s, max_w))
}

fn state_color(st: State) -> &'static str {
    match st {
        State::New => C_NEW,
        State::In => C_IN,
        State::Out => C_OUT,
        _ => INK,
    }
}

/// A state mark drawn as a shape (fonts don't carry these glyphs).
fn mark(st: State, x: f64, y: f64, s: f64) -> String {
    let c = state_color(st);
    match st {
        State::In => format!("<path d=\"M{x} {}L{} {}H{}Z\" fill=\"{c}\"/>", y - s, x + s, y + s * 0.8, x - s),
        State::Out => format!("<path d=\"M{x} {}L{} {}H{}Z\" fill=\"{c}\"/>", y + s, x + s, y - s * 0.8, x - s),
        State::New => format!("<path d=\"M{x} {}L{} {y}L{x} {}L{} {y}Z\" fill=\"{c}\"/>", y - s, x + s, y + s, x - s),
        State::Retired => format!(
            "<path d=\"M{} {}L{} {}M{} {}L{} {}\" stroke=\"{INK2}\" stroke-width=\"{}\"/>",
            x - s * 0.7,
            y - s * 0.7,
            x + s * 0.7,
            y + s * 0.7,
            x + s * 0.7,
            y - s * 0.7,
            x - s * 0.7,
            y + s * 0.7,
            s * 0.4
        ),
        _ => format!("<circle cx=\"{x}\" cy=\"{y}\" r=\"{}\" fill=\"{c}\"/>", s * 0.8),
    }
}

/// The radar, drawn with inline colours for resvg. `focus` highlights one blip and dims the rest.
fn radar(ctx: &Ctx, rv: &RadarView, e: usize, cx: f64, cy: f64, s: f64, focus: Option<usize>) -> String {
    let cs: Vec<_> = ctx.m.site.quadrants.iter().map(|q| q.position).collect();
    let pos = layout::layout(rv, &cs, e);
    let mut o = format!("<g transform=\"translate({cx} {cy}) scale({s})\">");
    for k in (0..4).rev() {
        let _ = write!(o, "<circle r=\"{}\" fill=\"{}\" stroke=\"{RULE}\" stroke-width=\"{}\"/>", layout::RING_OUTER[k], BANDS[k], 1.5 / s);
    }
    let mut ticks = String::new();
    for i in 0..72 {
        let a = i as f64 * 5.0_f64.to_radians();
        let len = if i % 6 == 0 { 14.0 } else { 6.0 };
        let _ = write!(ticks, "M{:.1} {:.1}L{:.1} {:.1}", a.cos() * R, a.sin() * R, a.cos() * (R + len), a.sin() * (R + len));
    }
    let _ = write!(o, "<path d=\"{ticks}\" stroke=\"{INK3}\" stroke-width=\"{}\"/><path d=\"M-{R} 0H{R}M0 -{R}V{R}\" stroke=\"{RULE}\" stroke-width=\"{}\"/>", 1.5 / s, 1.5 / s);
    let fq = focus.map(|i| rv.blips[i].q);
    for (i, p) in pos.iter().enumerate() {
        let Some(p) = p else { continue };
        let b = &rv.blips[i];
        // The first edition is the baseline, not a wall of "new". The focus keeps its lock brackets.
        let st = if e == 0 { State::Steady } else { b.state_at(e) };
        let is_focus = focus == Some(i);
        let dim = focus.is_some() && !is_focus;
        // Trail for moves this edition (or the whole history for the focus).
        if (e == rv.latest() && matches!(st, State::In | State::Out)) || is_focus {
            let t = layout::trail(rv, i, *p, if is_focus { 99 } else { 3 });
            if t.len() > 1 {
                let col = if matches!(st, State::In | State::Out) { state_color(st) } else { INK3 };
                let pts: Vec<String> = t.iter().map(|(x, y, _)| format!("{x:.1},{y:.1}")).collect();
                let _ = write!(o, "<polyline points=\"{}\" fill=\"none\" stroke=\"{col}\" stroke-width=\"{}\"/>", pts.join(" "), if is_focus { 4.0 } else { 3.0 });
                for (x, y, _) in &t[..t.len() - 1] {
                    let _ = write!(o, "<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"5\" fill=\"{BG}\" stroke=\"{col}\" stroke-width=\"2.5\"/>");
                }
            }
        }
        let col = if dim { INK3 } else { state_color(st) };
        let r = if is_focus { 14.0 } else { 9.0 };
        let op = if dim { if Some(b.q) == fq { 0.75 } else { 0.35 } } else { 1.0 };
        if st == State::New && !dim {
            let _ = write!(o, "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{}\" fill=\"none\" stroke=\"{C_NEW}\" stroke-width=\"3\"/>", p.x, p.y, r + 7.0);
        }
        let _ = write!(o, "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{r}\" fill=\"{col}\" stroke=\"{BG}\" stroke-width=\"3\" opacity=\"{op}\"/>", p.x, p.y);
        if is_focus {
            let _ = write!(
                o,
                "<path transform=\"translate({:.1} {:.1})\" d=\"M-28 -16V-28H-16M16 -28H28V-16M28 16V28H16M-16 28H-28V16\" fill=\"none\" stroke=\"{INK}\" stroke-width=\"4\"/>",
                p.x, p.y
            );
        }
    }
    o.push_str("</g>");
    o
}

fn frame(body: &str) -> String {
    format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1200\" height=\"630\" viewBox=\"0 0 1200 630\"><rect width=\"1200\" height=\"630\" fill=\"{BG}\"/>{body}</svg>")
}

fn wordmark(x: f64, y: f64) -> String {
    format!(
        "<g transform=\"translate({x} {})\" fill=\"none\" stroke=\"{INK}\" stroke-width=\"2.4\"><circle cx=\"14\" cy=\"14\" r=\"12.5\"/><circle cx=\"14\" cy=\"14\" r=\"7\"/><path d=\"M14 1.5V26.5M1.5 14H26.5\"/><circle cx=\"19\" cy=\"9\" r=\"3.2\" fill=\"{C_NEW}\" stroke=\"none\"/></g><text x=\"{}\" y=\"{y}\" {DISPLAY} font-size=\"24\" letter-spacing=\"1.5\" fill=\"{INK}\">PRODUCT RADAR</text>",
        y - 22.0,
        x + 42.0
    )
}

fn footer(ctx: &Ctx, rv: &RadarView, e: usize) -> String {
    let host = ctx.base_url.trim_start_matches("https://");
    let d = rv.dates[e];
    format!("<text x=\"72\" y=\"574\" {MONO} font-size=\"19\" fill=\"{INK2}\">{} · {}</text>", esc(host), esc(&crate::html::edition_name(e, d)))
}

fn lines(table_display: bool, lines: &[String], x: f64, y: f64, size: f64, lh: f64, fill: &str) -> String {
    let attrs = if table_display { DISPLAY } else { TEXT };
    let mut o = format!("<text {attrs} font-size=\"{size}\" fill=\"{fill}\">");
    for (i, l) in lines.iter().enumerate() {
        let _ = write!(o, "<tspan x=\"{x}\" y=\"{:.1}\">{}</tspan>", y + i as f64 * lh, esc(l));
    }
    o.push_str("</text>");
    o
}

fn ring_chip(ctx: &Ctx, k: Option<usize>, x: f64, y: f64) -> (String, f64) {
    let label = match k {
        Some(k) => ctx.m.ring_name(k).to_string(),
        None => "Retired".into(),
    };
    let size = 24.0;
    let w = width(&W_TEXT, &label, size) + 36.0 + 44.0;
    // Same chip as on the site: the ring's tint, its meter in the ring's mark colour, ink text.
    let (fill, mk) = match k {
        Some(k) => (RING_BG[k], RINGS[k]),
        None => (BG, INK3),
    };
    let mut o = format!("<rect x=\"{x}\" y=\"{}\" width=\"{w:.1}\" height=\"46\" rx=\"4\" fill=\"{fill}\" stroke=\"{mk}\" stroke-opacity=\"0.5\" stroke-width=\"1.5\"/>", y - 32.0);
    // Signal meter: filled bars = strength of recommendation.
    for j in 0..4 {
        let on = k.is_some_and(|k| j < 4 - k);
        let _ = write!(
            o,
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"6\" height=\"{}\" fill=\"{}\" opacity=\"{}\"/>",
            x + 16.0 + j as f64 * 9.0,
            y - 2.0 - (6.0 + j as f64 * 4.0),
            6.0 + j as f64 * 4.0,
            mk,
            if on { 1.0 } else { 0.25 }
        );
    }
    let _ = write!(o, "<text x=\"{:.1}\" y=\"{y}\" {TEXT} font-size=\"{size}\" fill=\"{}\">{}</text>", x + 18.0 + 44.0, if k.is_some() { INK } else { INK2 }, esc(&label));
    (o, w)
}

pub fn entry_card(ctx: &Ctx, eid: &str) -> String {
    let m = ctx.m;
    let rv = m.main();
    let en = m.entries.iter().find(|x| x.id == eid).unwrap();
    let q = m.site.quadrants.iter().position(|x| x.id == en.quadrant).unwrap();
    let e = rv.latest();
    let mut o = wordmark(72.0, 96.0);
    let bi = rv.blips.iter().position(|b| b.e.id == eid);
    if let Some(i) = bi {
        o.push_str(&radar(ctx, rv, (0..=e).rev().find(|&x| rv.blips[i].rings[x].is_some()).unwrap_or(e), 925.0, 330.0, 0.74, Some(i)));
    } else {
        o.push_str(&radar(ctx, rv, e, 925.0, 330.0, 0.74, None));
    }
    let _ = write!(o, "<text x=\"72\" y=\"196\" {MONO} font-size=\"21\" fill=\"{INK2}\">{}</text>", esc(&m.quad_name(q).to_uppercase()));
    let (size, ls) = fit(&W_DISPLAY, &en.name, &[92.0, 84.0, 76.0, 68.0, 60.0, 52.0, 46.0], 560.0, 2);
    let lh = size * 1.02;
    let top = 196.0 + 18.0 + size * 0.86;
    o.push_str(&lines(true, &ls, 72.0, top, size, lh, INK));
    let y = top + lh * (ls.len() as f64 - 1.0) + 84.0;
    if let Some(i) = bi {
        let b = &rv.blips[i];
        let (chip, w) = ring_chip(ctx, b.ring_now(), 72.0, y);
        o.push_str(&chip);
        let lc = b.last_change();
        let st = b.state_at(lc);
        // Edition 01 is the starting point: nothing on it is marked as new.
        let changed = lc == e && e > 0;
        let text =
            if changed { format!("{} {}", st.label().to_uppercase(), rv.dates[lc].short().to_uppercase()) } else { format!("SINCE {}", b.first_seen().short().to_uppercase()) };
        let x = 72.0 + w + 26.0;
        if changed {
            o.push_str(&mark(st, x + 10.0, y - 9.0, 10.0));
        }
        let _ = write!(o, "<text x=\"{}\" y=\"{y}\" {MONO} font-size=\"20\" fill=\"{INK2}\">{}</text>", x + if changed { 30.0 } else { 0.0 }, esc(&text));
    }
    o.push_str(&footer(ctx, rv, e));
    frame(&o)
}

fn page_card(ctx: &Ctx, rv: &RadarView, e: usize, eyebrow: &str, title: &str, sub: &str) -> String {
    let mut o = wordmark(72.0, 96.0);
    o.push_str(&radar(ctx, rv, e, 925.0, 330.0, 0.74, None));
    let _ = write!(o, "<text x=\"72\" y=\"196\" {MONO} font-size=\"21\" fill=\"{INK2}\">{}</text>", esc(&eyebrow.to_uppercase()));
    let (size, ls) = fit(&W_DISPLAY, title, &[64.0, 58.0, 52.0, 46.0, 40.0], 560.0, 4);
    let lh = size * 1.06;
    let top = 196.0 + 20.0 + size * 0.86;
    o.push_str(&lines(true, &ls, 72.0, top, size, lh, INK));
    let y = top + lh * (ls.len() as f64 - 1.0) + 56.0;
    let sl = wrap(&W_TEXT, sub, 24.0, 540.0);
    o.push_str(&lines(false, &sl[..sl.len().min(3)], 72.0, y, 24.0, 32.0, INK2));
    o.push_str(&footer(ctx, ctx.m.main(), ctx.m.main().latest()));
    frame(&o)
}

pub fn home_card(ctx: &Ctx) -> String {
    let rv = ctx.m.main();
    let e = rv.latest();
    let mut o = wordmark(72.0, 96.0);
    o.push_str(&radar(ctx, rv, e, 925.0, 330.0, 0.74, None));
    let (size, ls) = fit(&W_DISPLAY, "A weekly radar of the tools for building a business with AI agents.", &[56.0, 50.0, 44.0], 580.0, 4);
    let lh = size * 1.06;
    o.push_str(&lines(true, &ls, 72.0, 190.0 + size * 0.5, size, lh, INK));
    let mut x = 72.0;
    let y = 190.0 + size * 0.5 + lh * (ls.len() as f64 - 1.0) + 74.0;
    if e == 0 {
        // Edition 01 is the starting point: entries per ring, no "new" counts.
        let t = ring_line(ctx, rv, e);
        let _ = write!(o, "<text x=\"72\" y=\"{y}\" {MONO} font-size=\"20\" fill=\"{INK2}\">{}</text>", esc(&t));
        o.push_str(&footer(ctx, rv, e));
        return frame(&o);
    }
    for st in [State::New, State::In, State::Out] {
        let n = rv.count(e, st);
        o.push_str(&mark(st, x + 10.0, y - 9.0, 10.0));
        let t = format!("{n} {}", st.label().to_uppercase());
        let _ = write!(o, "<text x=\"{}\" y=\"{y}\" {MONO} font-size=\"20\" fill=\"{INK2}\">{}</text>", x + 28.0, esc(&t));
        x += 28.0 + mono_w(&t, 20.0) + 30.0;
    }
    o.push_str(&footer(ctx, rv, e));
    frame(&o)
}

/// Card for the curated radar's own page: the scope, its name and how many entries sit in each ring.
pub fn radar_card(ctx: &Ctx, title: &str) -> String {
    let rv = ctx.m.main();
    let e = rv.latest();
    let mut o = wordmark(72.0, 96.0);
    o.push_str(&radar(ctx, rv, e, 925.0, 330.0, 0.74, None));
    let _ = write!(o, "<text x=\"72\" y=\"196\" {MONO} font-size=\"21\" fill=\"{INK2}\">{} ENTRIES IN FOUR QUADRANTS</text>", rv.on_radar(e).count());
    let (size, ls) = fit(&W_DISPLAY, title, &[64.0, 58.0, 52.0], 560.0, 2);
    let lh = size * 1.06;
    let top = 196.0 + 20.0 + size * 0.86;
    o.push_str(&lines(true, &ls, 72.0, top, size, lh, INK));
    let y = top + lh * (ls.len() as f64 - 1.0) + 84.0;
    o.push_str(&ring_grid(ctx, rv, e, y));
    o.push_str(&footer(ctx, rv, e));
    frame(&o)
}

fn ring_counts(rv: &RadarView, e: usize) -> [usize; 4] {
    let mut n = [0usize; 4];
    for (_, b) in rv.on_radar(e) {
        if let Some(k) = b.rings[e] {
            n[k] += 1;
        }
    }
    n
}

/// "19 ADOPT · 14 TRIAL · 2 ASSESS · 6 HOLD"
fn ring_line(ctx: &Ctx, rv: &RadarView, e: usize) -> String {
    ring_counts(rv, e).iter().enumerate().map(|(k, c)| format!("{c} {}", ctx.m.ring_name(k).to_uppercase())).collect::<Vec<_>>().join(" · ")
}

/// Ring chips with their counts, two rings per row, first baseline at `y`.
fn ring_grid(ctx: &Ctx, rv: &RadarView, e: usize, y: f64) -> String {
    let mut o = String::new();
    for (k, c) in ring_counts(rv, e).iter().enumerate() {
        let x = 72.0 + (k % 2) as f64 * 270.0;
        let yy = y + (k / 2) as f64 * 64.0;
        let (chip, w) = ring_chip(ctx, Some(k), x, yy);
        o.push_str(&chip);
        let _ = write!(o, "<text x=\"{:.1}\" y=\"{yy}\" {MONO} font-size=\"26\" fill=\"{INK}\">{c}</text>", x + w + 16.0);
    }
    o
}

pub fn edition_card(ctx: &Ctx, e: usize) -> String {
    let m = ctx.m;
    let rv = m.main();
    let ed = rv.edition(e);
    let mut o = wordmark(72.0, 96.0);
    o.push_str(&radar(ctx, rv, e, 925.0, 330.0, 0.74, None));
    let _ = write!(o, "<text x=\"72\" y=\"186\" {MONO} font-size=\"21\" fill=\"{INK2}\">WHAT MOVED</text>");
    let (size, ls) = fit(&W_DISPLAY, &ed.title, &[56.0, 50.0, 44.0, 38.0], 560.0, 2);
    let lh = size * 1.06;
    let top = 186.0 + 16.0 + size * 0.86;
    o.push_str(&lines(true, &ls, 72.0, top, size, lh, INK));
    let mut y = top + lh * (ls.len() as f64 - 1.0) + 62.0;
    if e == 0 {
        // Edition 01 is the starting point: the radar it starts from, ring by ring, instead of a list of "new" rows.
        let _ = write!(o, "<text x=\"72\" y=\"{y}\" {MONO} font-size=\"20\" fill=\"{INK2}\">{} ENTRIES PLACED</text>", rv.on_radar(0).count());
        o.push_str(&ring_grid(ctx, rv, 0, y + 72.0));
        o.push_str(&footer(ctx, rv, e));
        return frame(&o);
    }
    let mvs = rv.moves_at(e);
    for mv in mvs.iter().take(if ls.len() > 1 { 4 } else { 5 }) {
        let b = &rv.blips[mv.blip];
        o.push_str(&mark(mv.state, 82.0, y - 9.0, 9.0));
        let name = b.e.name.to_uppercase();
        let _ = write!(o, "<text x=\"106\" y=\"{y}\" {TEXT} font-size=\"24\" fill=\"{INK}\">{}</text>", esc(&b.e.name));
        let rings = match (mv.from, mv.to) {
            (None, Some(t)) => format!("NEW · {}", m.ring_name(t).to_uppercase()),
            (Some(f), None) => format!("RETIRED · WAS {}", m.ring_name(f).to_uppercase()),
            (Some(f), Some(t)) => format!("{} → {}", m.ring_name(f).to_uppercase(), m.ring_name(t).to_uppercase()),
            _ => String::new(),
        };
        let nx = 106.0 + width(&W_TEXT, &b.e.name, 24.0) + 16.0;
        let _ = name;
        let _ = write!(o, "<text x=\"{nx:.1}\" y=\"{y}\" {MONO} font-size=\"17\" fill=\"{INK2}\">{}</text>", esc(&rings));
        y += 40.0;
    }
    if mvs.len() > 5 {
        let _ = write!(o, "<text x=\"106\" y=\"{y}\" {MONO} font-size=\"17\" fill=\"{INK3}\">+{} MORE</text>", mvs.len() - 5);
    }
    o.push_str(&footer(ctx, rv, e));
    frame(&o)
}

pub fn generic_card(ctx: &Ctx, eyebrow: &str, title: &str, sub: &str) -> String {
    let rv = ctx.m.main();
    page_card(ctx, rv, rv.latest(), eyebrow, title, sub)
}

pub fn community_card(ctx: &Ctx, ri: usize) -> String {
    let m = ctx.m;
    let rv = &m.radars[ri];
    let main = m.main();
    let e = rv.latest();
    let (mut same, mut shared) = (0, 0);
    for (_, b) in rv.on_radar(e) {
        if let Some(o) = main.find(&b.e.id).and_then(|x| x.ring_now()) {
            shared += 1;
            if Some(o) == b.rings[e] {
                same += 1;
            }
        }
    }
    let sub = format!("By {}. {same} of {shared} shared entries sit in the same ring as on {}.", rv.r.owner, m.curated());
    page_card(ctx, rv, e, "Community radar", &rv.r.title, &sub)
}

/// README badge. System monospace with textLength, so it renders the same everywhere.
pub fn badge(ctx: &Ctx, eid: &str) -> String {
    let rv = ctx.m.main();
    let ring = rv.find(eid).map(|b| b.ring_now());
    let (label, k) = match ring {
        Some(Some(k)) => (ctx.m.ring_name(k).to_uppercase(), Some(k)),
        Some(None) => ("RETIRED".to_string(), None),
        None => ("LISTED".to_string(), None),
    };
    let lw = 138.0;
    let w = 236.0;
    let mut bars = String::new();
    for j in 0..4 {
        let on = k.is_some_and(|k| j < 4 - k);
        let h = 5.0 + j as f64 * 3.0;
        let _ = write!(
            bars,
            "<rect x=\"{}\" y=\"{}\" width=\"3.5\" height=\"{h}\" fill=\"#eef1f2\" opacity=\"{}\"/>",
            lw + 12.0 + j as f64 * 5.5,
            20.0 - h,
            if on { 1.0 } else { 0.3 }
        );
    }
    let tw = label.len() as f64 * 7.2;
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"28\" viewBox=\"0 0 {w} 28\" role=\"img\" aria-label=\"Product Radar: {label}\"><title>Product Radar: {label}</title>\
<rect width=\"{w}\" height=\"28\" rx=\"4\" fill=\"#0f1b24\"/><rect x=\"1\" y=\"1\" width=\"{lw}\" height=\"26\" rx=\"3\" fill=\"#eef1f2\"/>\
<g fill=\"none\" stroke=\"#0f1b24\" stroke-width=\"1.3\"><circle cx=\"15\" cy=\"14\" r=\"7.5\"/><circle cx=\"15\" cy=\"14\" r=\"4\"/><path d=\"M15 6.5V21.5M7.5 14H22.5\"/></g><circle cx=\"18.2\" cy=\"10.8\" r=\"2\" fill=\"#c5157a\"/>\
<g font-family=\"ui-monospace,SFMono-Regular,Menlo,Consolas,'Liberation Mono',monospace\" font-size=\"11\" font-weight=\"600\"><text x=\"29\" y=\"18\" fill=\"#0f1b24\" textLength=\"100\" lengthAdjust=\"spacingAndGlyphs\">PRODUCT RADAR</text>\
<text x=\"{}\" y=\"18\" fill=\"#eef1f2\" textLength=\"{tw}\" lengthAdjust=\"spacingAndGlyphs\">{label}</text></g>{bars}</svg>",
        lw + 42.0
    )
}

pub const FAVICON: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 32 32\"><style>.i{fill:none;stroke:#0f1b24;stroke-width:2.2}.b{fill:#c5157a}@media (prefers-color-scheme:dark){.i{stroke:#e4eaee}.b{fill:#e0468f}}</style><circle class=\"i\" cx=\"16\" cy=\"16\" r=\"13.5\"/><circle class=\"i\" cx=\"16\" cy=\"16\" r=\"7.5\"/><path class=\"i\" d=\"M16 2.5v27M2.5 16h27\"/><circle class=\"b\" cx=\"21.5\" cy=\"10.5\" r=\"3.6\"/></svg>";

pub fn touch_icon() -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"180\" height=\"180\" viewBox=\"0 0 180 180\"><rect width=\"180\" height=\"180\" fill=\"{BG}\"/><g fill=\"none\" stroke=\"{INK}\" stroke-width=\"7\"><circle cx=\"90\" cy=\"90\" r=\"62\"/><circle cx=\"90\" cy=\"90\" r=\"34\"/><path d=\"M90 28V152M28 90H152\"/></g><circle cx=\"116\" cy=\"62\" r=\"15\" fill=\"{C_NEW}\"/></svg>"
    )
}

pub struct Renderer {
    opt: usvg::Options<'static>,
}

impl Renderer {
    pub fn new() -> Renderer {
        let mut opt = usvg::Options::default();
        let db = opt.fontdb_mut();
        for font in [
            &include_bytes!("../assets/og-fonts/Archivo-ExpandedExtraBold.ttf")[..],
            &include_bytes!("../assets/og-fonts/Archivo-Medium.ttf")[..],
            &include_bytes!("../assets/og-fonts/MartianMono-SemiCondensedMedium.ttf")[..],
        ] {
            db.load_font_data(font.to_vec());
        }
        Renderer { opt }
    }

    pub fn png(&self, svg: &str) -> Result<Vec<u8>, String> {
        let tree = usvg::Tree::from_str(svg, &self.opt).map_err(|e| e.to_string())?;
        let size = tree.size().to_int_size();
        let mut pm = tiny_skia::Pixmap::new(size.width(), size.height()).ok_or("empty image")?;
        resvg::render(&tree, tiny_skia::Transform::default(), &mut pm.as_mut());
        pm.encode_png().map_err(|e| e.to_string())
    }

    pub fn families(&self) -> Vec<String> {
        let mut v: Vec<String> = self.opt.fontdb.faces().map(|f| format!("{} {:?} {:?}", f.families[0].0, f.weight, f.stretch)).collect();
        v.sort();
        v
    }
}
