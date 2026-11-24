//! Page bodies. Each function returns a `Page` for the shell in html.rs.

use crate::html::{Ctx, Page, esc, inline, paras};
use crate::layout;
use crate::model::Kind;
use crate::svg;
use crate::view::{BlipView, MoveView, RadarView, State};
use std::fmt::Write;

pub fn ring_chip(ctx: &Ctx, k: usize) -> String {
    let mut s = format!("<span class=\"ring r{k}\"><span class=\"meter\" aria-hidden=\"true\">");
    for j in 0..4 {
        s.push_str(if j < 4 - k { "<i class=\"on\"></i>" } else { "<i></i>" });
    }
    let _ = write!(s, "</span>{}</span>", esc(ctx.m.ring_name(k)));
    s
}

fn meter(k: usize) -> String {
    let mut s = String::from("<span class=\"meter\" aria-hidden=\"true\">");
    for j in 0..4 {
        s.push_str(if j < 4 - k { "<i class=\"on\"></i>" } else { "<i></i>" });
    }
    s.push_str("</span>");
    s
}

fn mark(st: State) -> String {
    format!("<span class=\"mk {}\" aria-hidden=\"true\">{}</span>", st.class(), st.glyph())
}

fn move_rings(ctx: &Ctx, mv: &MoveView) -> String {
    let r = |k: usize| esc(ctx.m.ring_name(k));
    match (mv.from, mv.to) {
        (None, Some(t)) => format!("New in {}", r(t)),
        (Some(f), None) => format!("Retired from {}", r(f)),
        (Some(f), Some(t)) => format!("{} → {}", r(f), r(t)),
        _ => String::new(),
    }
}

fn moves_list(ctx: &Ctx, from: &str, rv: &RadarView, e: usize, notes: bool, limit: usize) -> String {
    let mut mvs = rv.moves_at(e);
    mvs.truncate(limit);
    if mvs.is_empty() {
        return "<p class=\"quiet\">No changes in this edition.</p>".into();
    }
    let mut o = String::from("<ol class=\"moves\">");
    for mv in &mvs {
        let b = &rv.blips[mv.blip];
        let _ = write!(
            o,
            "<li class=\"mv {cls}\" data-i=\"{i}\">{mk}<span class=\"mv-main\"><a href=\"{href}\">{name}</a><span class=\"mv-rings\">{rings}</span></span>",
            cls = mv.state.class(),
            i = mv.blip,
            mk = mark(mv.state),
            href = ctx.href(from, &format!("e/{}/", b.e.id)),
            name = esc(&b.e.name),
            rings = move_rings(ctx, mv),
        );
        if notes && !mv.note.is_empty() {
            let _ = write!(o, "<span class=\"mv-note\">{}</span>", inline(mv.note));
        }
        o.push_str("</li>");
    }
    o.push_str("</ol>");
    o
}

/// The one name of an edition, on every page and card: "Edition 01 · 12 Oct 2026".
fn edition_stamp(rv: &RadarView, e: usize) -> String {
    crate::html::edition_name(e, rv.dates[e])
}

fn tally(ctx: &Ctx, rv: &RadarView, e: usize) -> String {
    let on = rv.on_radar(e).count();
    let mut o = format!("<ul class=\"tally\" aria-label=\"This edition in numbers\"><li class=\"t-all\"><b>{on}</b> on the radar</li>");
    for st in [State::New, State::In, State::Out, State::Retired] {
        let n = rv.count(e, st);
        let _ = write!(o, "<li class=\"{}\">{}<b>{n}</b> {}</li>", st.class(), mark(st), st.label().to_lowercase());
    }
    let _ = ctx;
    o.push_str("</ul>");
    o
}

/// JSON the client script reads: per-edition positions and states, compact.
fn radar_json(ctx: &Ctx, rv: &RadarView) -> String {
    let cs: Vec<_> = ctx.m.site.quadrants.iter().map(|q| q.position).collect();
    let per: Vec<Vec<Option<layout::Pos>>> = (0..rv.dates.len()).map(|e| layout::layout(rv, &cs, e)).collect();
    let mut o = String::from("{\"e\":[");
    for (e, d) in rv.dates.iter().enumerate() {
        let _ = write!(o, "{}{{\"s\":\"{}\",\"t\":{}}}", if e > 0 { "," } else { "" }, edition_stamp(rv, e), json_str(&rv.edition(e).title));
        let _ = d;
    }
    o.push_str("],\"r\":[");
    o.push_str(&ctx.m.site.rings.iter().map(|r| json_str(&r.name)).collect::<Vec<_>>().join(","));
    o.push_str("],\"q\":[");
    o.push_str(&ctx.m.site.quadrants.iter().map(|q| json_str(&q.name)).collect::<Vec<_>>().join(","));
    o.push_str("],\"c\":[");
    o.push_str(&cs.iter().map(|c| format!("\"{}\"", corner_code(*c))).collect::<Vec<_>>().join(","));
    o.push_str("],\"b\":[");
    for (i, b) in rv.blips.iter().enumerate() {
        let _ = write!(o, "{}{{\"id\":{},\"n\":{},\"q\":{},\"p\":[", if i > 0 { "," } else { "" }, json_str(&b.e.id), json_str(&b.e.name), b.q);
        for (e, row) in per.iter().enumerate() {
            if e > 0 {
                o.push(',');
            }
            match (row[i], b.rings[e]) {
                (Some(p), Some(k)) => {
                    let st = if e == 0 { State::Steady } else { b.state_at(e) };
                    let _ = write!(o, "[{:.1},{:.1},{k},{}]", p.x, p.y, st.code());
                }
                _ => o.push('0'),
            }
        }
        o.push_str("]}");
    }
    o.push_str("]}");
    o
}

fn corner_code(c: crate::model::Corner) -> &'static str {
    match c {
        crate::model::Corner::TopLeft => "tl",
        crate::model::Corner::TopRight => "tr",
        crate::model::Corner::BottomLeft => "bl",
        crate::model::Corner::BottomRight => "br",
    }
}

pub fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '<' => o.push_str("\\u003c"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Static scope beside a side panel (community radars).
fn deck(ctx: &Ctx, from: &str, rv: &RadarView, side: &str) -> String {
    format!(
        "<section class=\"deck wrap\" aria-label=\"Radar\"><div class=\"scope-col\"><figure class=\"scope\">{}<figcaption class=\"sr\">Radar of {} entries in four quadrants and four rings. The same entries are listed below.</figcaption></figure></div>{side}</section>",
        svg::scope(ctx, rv, from, false),
        rv.on_radar(rv.latest()).count()
    )
}

/// The scope as a picture: blips become plain groups, so the whole scope can sit inside one link.
fn scope_picture(ctx: &Ctx, rv: &RadarView, from: &str) -> String {
    let mut s = svg::scope(ctx, rv, from, false).replace("<a ", "<g ").replace("</a>", "</g>").replace(" tabindex=\"-1\"", "");
    while let Some(i) = s.find(" href=\"") {
        let j = s[i + 7..].find('"').map_or(s.len(), |j| i + 7 + j + 1);
        s.replace_range(i..j, "");
    }
    s
}

/// Entries per ring at edition `e`, for one quadrant or all of them.
fn ring_counts(rv: &RadarView, e: usize, q: Option<usize>) -> [usize; 4] {
    let mut n = [0; 4];
    for (_, b) in rv.on_radar(e) {
        if let Some(k) = b.rings[e]
            && q.is_none_or(|q| b.q == q)
        {
            n[k] += 1;
        }
    }
    n
}

fn ring_count_list(ctx: &Ctx, n: &[usize; 4], class: &str) -> String {
    let mut o = format!("<ul class=\"rcount {class}\">");
    for (k, c) in n.iter().enumerate() {
        let _ = write!(o, "<li>{}<b>{c}</b></li>", ring_chip(ctx, k));
    }
    o.push_str("</ul>");
    o
}

/// Edition 01 is the starting point: how many entries it placed, ring by ring. Nothing on it counts as a move.
fn baseline_sum(ctx: &Ctx, rv: &RadarView) -> String {
    format!("<div class=\"m-sum\"><p class=\"m-big\"><b>{}</b> entries placed</p>{}</div>", rv.on_radar(0).count(), ring_count_list(ctx, &ring_counts(rv, 0, None), "row"))
}

/// Shared entries of a radar, and how many of them sit in the same ring as on the curated radar.
fn agreement(main: &RadarView, rv: &RadarView) -> (usize, usize) {
    let (e, me) = (rv.latest(), main.latest());
    let (mut same, mut shared) = (0, 0);
    for (_, b) in rv.on_radar(e) {
        if let Some(o) = main.find(&b.e.id).and_then(|x| x.rings[me]) {
            shared += 1;
            if b.rings[e] == Some(o) {
                same += 1;
            }
        }
    }
    (same, shared)
}

fn index_lists(ctx: &Ctx, from: &str, rv: &RadarView) -> String {
    let e = rv.latest();
    let mut o = String::from("<div class=\"quads\">");
    for (q, quad) in ctx.m.site.quadrants.iter().enumerate() {
        let n = rv.on_radar(e).filter(|(_, b)| b.q == q).count();
        let _ = write!(
            o,
            "<section class=\"quad\" id=\"q-{}\"><h3><span>{}</span><span class=\"count\">{n}</span></h3><p class=\"quad-d\">{}</p>",
            quad.id,
            esc(&quad.name),
            esc(&quad.description)
        );
        for k in 0..4 {
            let items: Vec<(usize, &BlipView)> = rv.on_radar(e).filter(|(_, b)| b.q == q && b.rings[e] == Some(k)).collect();
            if items.is_empty() {
                continue;
            }
            let _ = write!(o, "<div class=\"ringrow r{k}\"><h4>{}</h4><ul>", ring_chip(ctx, k));
            for (i, b) in items {
                // Edition 01 is the baseline: nothing on it counts as new.
                let st = if e == 0 { State::Steady } else { b.state_at(e) };
                let m = if matches!(st, State::Steady) { String::new() } else { format!(" {}<span class=\"sr\">{}</span>", mark(st), st.label()) };
                let _ = write!(o, "<li data-i=\"{i}\"><a href=\"{}\">{}</a>{m}</li>", ctx.href(from, &format!("e/{}/", b.e.id)), esc(&b.e.name));
            }
            o.push_str("</ul></div>");
        }
        o.push_str("</section>");
    }
    o.push_str("</div>");
    o
}

/// Home: what Product Radar is, for whom, and the ways into it.
pub fn home(ctx: &Ctx) -> Page {
    let m = ctx.m;
    let rv = m.main();
    let e = rv.latest();
    let from = "";
    let total = rv.on_radar(e).count();
    let radar = ctx.href(from, "radar/");
    let mut b = String::new();

    let _ = write!(
        b,
        "<section class=\"intro wrap\"><div class=\"i-copy\"><h1>A weekly radar of the tools for building a business with AI agents.</h1>\
<p class=\"lede\">Konstantin Kaufmann builds and runs his products with AI agents. Here he places each model, tool and practice he has used in a ring, from Adopt to Hold.</p>\
<p>He writes for solo founders, for product managers who now build, and for engineers who are moving towards product.</p>\
<p>Each week, agents turn the decisions from his working sessions into a new edition. Each change says why.</p>\
<p class=\"i-cta\"><a class=\"btn\" href=\"{radar}\">Open {name}</a><a class=\"btn ghost\" href=\"{build}\">Build your own radar</a></p></div>\
<a class=\"i-scope\" href=\"{radar}\"><figure class=\"scope pic{base}\">{pic}</figure><span class=\"i-cap\"><b>{name}</b><span>{stamp}</span><span>{total} entries</span></span></a></section>",
        name = esc(m.curated()),
        build = ctx.href(from, "build/"),
        base = if e == 0 { " baseline" } else { "" },
        pic = scope_picture(ctx, rv, from),
        stamp = edition_stamp(rv, e),
    );

    // How to read a radar: the quadrant map mirrors the quadrants' places on the scope.
    let mut qmap = String::new();
    for (q, quad) in m.site.quadrants.iter().enumerate() {
        let n = rv.on_radar(e).filter(|(_, x)| x.q == q).count();
        let _ =
            write!(qmap, "<div class=\"c-{}\"><h4>{}</h4><p>{}</p><p class=\"qm-n\">{n} entries</p></div>", corner_code(quad.position), esc(&quad.name), esc(&quad.description));
    }
    let mut rings = String::new();
    for (k, r) in m.site.rings.iter().enumerate() {
        let _ = write!(rings, "<li class=\"r{k}\">{}<span>{}</span></li>", ring_chip(ctx, k), esc(&r.description));
    }
    let _ = write!(
        b,
        "<section class=\"sect wrap\" id=\"read\" aria-labelledby=\"read-h\"><h2 id=\"read-h\">How to read a radar</h2><div class=\"sect-body\">\
<p class=\"sect-lede\">Each entry is a dot on the radar. Its quadrant says what kind of thing it is. Its ring says what to do about it.</p>\
<div class=\"cols read\"><div><h3>Quadrants, as placed on the radar</h3><div class=\"qmap\">{qmap}</div></div>\
<div><h3>Rings, from the centre out</h3><ol class=\"rings\">{rings}</ol><p><a href=\"{about}#movement\">How the radar shows movement</a></p></div></div></div></section>",
        about = ctx.href(from, "about/"),
    );

    // Latest edition: counts and real moves only.
    let ed_href = ctx.href(from, &format!("moves/{}/", rv.dates[e]));
    let mut mv = format!("<p class=\"m-stamp\">{}</p>", edition_stamp(rv, e));
    if e == 0 {
        mv.push_str(&baseline_sum(ctx, rv));
    } else {
        let n = rv.moves_at(e).len();
        let _ = write!(mv, "<h3 class=\"m-title\"><a href=\"{ed_href}\">{}</a></h3>{}", esc(&rv.edition(e).title), tally(ctx, rv, e));
        mv.push_str(&moves_list(ctx, from, rv, e, true, 8));
        if n > 8 {
            let _ = write!(mv, "<p><a href=\"{ed_href}\">{} more changes in edition {:02}</a></p>", n - 8, e + 1);
        }
    }
    let _ = write!(
        b,
        "<section class=\"sect wrap\" id=\"moved\" aria-labelledby=\"moved-h\"><h2 id=\"moved-h\">Latest edition</h2><div class=\"sect-body moved\">{mv}<p class=\"links\"><a class=\"more\" href=\"{ed_href}\">Open edition {:02}</a><a class=\"more\" href=\"{}\">All editions</a><a class=\"more\" href=\"{}\">Atom feed</a></p></div></section>",
        e + 1,
        ctx.href(from, "moves/"),
        ctx.href(from, "feed.xml"),
    );

    b.push_str(&build_cta(ctx, from));

    // Community radars, each compared with the curated one.
    let mut comm = String::new();
    for r in m.radars.iter().skip(1) {
        let (same, shared) = agreement(rv, r);
        let ex = if r.r.example { "<span class=\"chip\">Example</span>" } else { "" };
        let _ = write!(
            comm,
            "<li><a href=\"{}\">{}</a><span class=\"by\">by {}</span>{ex}<span class=\"agree\"><b>{same}</b> of {shared} shared entries in the same ring</span></li>",
            ctx.href(from, &format!("r/{}/", r.r.id)),
            esc(&r.r.title),
            esc(&r.r.owner)
        );
    }
    if comm.is_empty() {
        comm.push_str("<li class=\"quiet\">No community radars yet.</li>");
    }
    let _ = write!(
        b,
        "<section class=\"sect wrap\" id=\"community\" aria-labelledby=\"comm-h\"><h2 id=\"comm-h\">Community radars</h2><div class=\"sect-body\">\
<p class=\"sect-lede\">Anyone can keep a radar of the tools they use. The site compares each one with {name}, entry by entry, so you see where people disagree.</p>\
<ul class=\"comm\">{comm}</ul><p>To publish yours here, export it from the builder as a file and open a pull request.</p></div></section>",
        name = esc(m.curated()),
    );

    let _ = write!(
        b,
        "<section class=\"sect wrap\" id=\"data\" aria-labelledby=\"data-h\"><h2 id=\"data-h\">Open data</h2><div class=\"sect-body\"><dl class=\"dl3\">\
<div><dt><a href=\"{}\">radar.json</a></dt><dd>Every entry, ring and change, as JSON. Free to use and quote.</dd></div>\
<div><dt><a href=\"{}\">llms.txt</a></dt><dd>A plain-text summary of the radar for language models.</dd></div>\
<div><dt><a href=\"{}\">Atom feed</a></dt><dd>One post per edition, with each move and its reason.</dd></div></dl></div></section>",
        ctx.href(from, "radar.json"),
        ctx.href(from, "llms.txt"),
        ctx.href(from, "feed.xml"),
    );

    Page {
        path: String::new(),
        title: m.site.title.clone(),
        desc: m.site.description.clone(),
        og: "og/home.png".into(),
        nav: "",
        class: "p-home",
        body: b,
        scripts: &["site.js"],
        head_extra: String::new(),
    }
}

/// The curated radar on its own page: the scope is the hero, the controls sit in a rail beside it.
pub fn radar(ctx: &Ctx) -> Page {
    let m = ctx.m;
    let rv = m.main();
    let e = rv.latest();
    let from = "radar/";
    let total = rv.on_radar(e).count();
    let mut b = String::new();
    let _ = write!(
        b,
        "<section class=\"rdeck wrap{}\" data-radar aria-labelledby=\"r-h\"><header class=\"r-head\"><h1 id=\"r-h\">{}</h1><p class=\"r-stamp\">{}</p><p class=\"r-lede\">{} Each one sits in a ring, from Adopt to Hold.</p></header>",
        if e == 0 { " baseline" } else { "" },
        esc(m.curated()),
        edition_stamp(rv, e),
        inline(&rv.r.description)
    );
    // The selector mirrors the scope: each quadrant button sits where its quadrant is.
    let _ = write!(
        b,
        "<div class=\"qsel\" role=\"group\" aria-label=\"Zoom to a quadrant\"><button type=\"button\" class=\"q-all\" data-q=\"-1\" aria-pressed=\"true\"><span class=\"q-n\">All quadrants</span><span class=\"q-c\">{total}</span></button>"
    );
    for (q, quad) in m.site.quadrants.iter().enumerate() {
        let n = rv.on_radar(e).filter(|(_, x)| x.q == q).count();
        let _ = write!(
            b,
            "<button type=\"button\" class=\"c-{}\" data-q=\"{q}\" aria-pressed=\"false\"><span class=\"q-n\">{}</span><span class=\"q-c\">{n}</span></button>",
            corner_code(quad.position),
            esc(&quad.name)
        );
    }
    b.push_str("</div>");
    let _ = write!(
        b,
        "<div class=\"scope-col\"><figure class=\"scope\"><div class=\"sweep\" aria-hidden=\"true\"></div>{}<figcaption class=\"sr\">Radar of {total} entries in four quadrants and four rings. The same entries are listed below.</figcaption><div class=\"tip\" hidden></div></figure></div>",
        svg::scope(ctx, rv, from, true)
    );
    b.push_str("<div class=\"r-info\"><div class=\"qinfo\" aria-live=\"polite\">");
    let _ = write!(
        b,
        "<div class=\"qd\" data-q=\"-1\"><p>Select a quadrant to zoom in. Select All quadrants to zoom out<span class=\"kbd-hint\">, or press Escape</span>.</p>{}</div>",
        ring_count_list(ctx, &ring_counts(rv, e, None), "")
    );
    for (q, quad) in m.site.quadrants.iter().enumerate() {
        let _ = write!(
            b,
            "<div class=\"qd\" data-q=\"{q}\" hidden><p><b>{}.</b> {}</p>{}</div>",
            esc(&quad.name),
            esc(&quad.description),
            ring_count_list(ctx, &ring_counts(rv, e, Some(q)), "")
        );
    }
    b.push_str("</div><ul class=\"legend small rkey\" aria-label=\"Key\">");
    for st in [State::New, State::In, State::Out, State::Retired] {
        let _ = write!(b, "<li>{}{}</li>", mark(st), st.label());
    }
    b.push_str("</ul>");
    if e == 0 {
        b.push_str("<p class=\"r-note\">Edition 01 is the starting point, so no entry is marked as new or moved yet.</p>");
    }
    if rv.dates.len() > 1 {
        let _ = write!(
            b,
            "<div class=\"replay\"><button class=\"play\" type=\"button\" aria-label=\"Replay every edition\"><svg viewBox=\"0 0 16 16\" aria-hidden=\"true\"><path class=\"i-play\" d=\"M4 2.5v11l9-5.5z\"/><path class=\"i-pause\" d=\"M4 3h3v10H4zM9 3h3v10H9z\"/></svg></button><input type=\"range\" id=\"edition\" min=\"0\" max=\"{e}\" value=\"{e}\" step=\"1\" aria-label=\"Edition\" aria-valuetext=\"{stamp}\"><output for=\"edition\">{stamp}</output></div>",
            stamp = edition_stamp(rv, e)
        );
    }
    let _ = write!(b, "</div></section><script type=\"application/json\" id=\"radar-data\">{}</script>", radar_json(ctx, rv));
    let _ = write!(
        b,
        "<section class=\"index wrap\" aria-labelledby=\"idx-h\"><header class=\"sec-head\"><h2 id=\"idx-h\">All {total} entries</h2><p>Grouped by quadrant, then by ring from Adopt to Hold. <a href=\"{}#read\">How to read a radar</a>.</p></header>{}</section>",
        ctx.href(from, ""),
        index_lists(ctx, from, rv)
    );
    let counts = ring_counts(rv, e, None);
    Page {
        path: from.into(),
        title: m.curated().into(),
        desc: format!(
            "{}: {total} models, tools, infrastructure and practices for building a business with AI agents. {} in Adopt, {} in Trial, {} in Assess and {} in Hold. Edition {:02}, {}.",
            m.curated(),
            counts[0],
            counts[1],
            counts[2],
            counts[3],
            e + 1,
            rv.dates[e].human()
        ),
        og: "og/radar.png".into(),
        nav: "radar",
        class: "p-radar-main",
        body: b,
        scripts: &["site.js"],
        head_extra: String::new(),
    }
}

fn build_cta(ctx: &Ctx, from: &str) -> String {
    format!(
        "<section class=\"sect wrap\" id=\"yours\" aria-labelledby=\"yours-h\"><h2 id=\"yours-h\">Build your own radar</h2><div class=\"sect-body cta2\"><div class=\"cta-in\">\
<p class=\"sect-lede\">Place the tools you use in your own rings. Start from {name} or from a blank radar.</p>\
<p>The share link holds your whole radar, so you need no account. Colour marks where your rings differ from {who}'s.</p>\
<a class=\"btn\" href=\"{}\">Build your radar</a></div><div class=\"cta-art\" aria-hidden=\"true\">{}</div></div></section>",
        ctx.href(from, "build/"),
        CTA_ART,
        name = esc(ctx.m.curated()),
        who = esc(ctx.m.curator()),
    )
}

/// Two overlapping scopes: yours and ours.
const CTA_ART: &str = r#"<svg viewBox="0 0 240 160"><g class="ca-a"><circle cx="90" cy="80" r="70"/><circle cx="90" cy="80" r="46"/><circle cx="90" cy="80" r="24"/></g><g class="ca-b"><circle cx="150" cy="80" r="70"/><circle cx="150" cy="80" r="46"/><circle cx="150" cy="80" r="24"/></g><circle class="ca-d1" cx="104" cy="66" r="5"/><circle class="ca-d2" cx="138" cy="94" r="5"/><path class="ca-l" d="M104 66L138 94"/></svg>"#;

pub fn entry(ctx: &Ctx, eid: &str) -> Page {
    let m = ctx.m;
    let e = m.entries.iter().find(|x| x.id == eid).unwrap();
    let from = format!("e/{}/", e.id);
    let main = m.main();
    let qi = m.site.quadrants.iter().position(|q| q.id == e.quadrant).unwrap();
    let mut b = String::new();
    let placed = main.blips.iter().position(|x| x.e.id == e.id);

    let _ = write!(
        b,
        "<article class=\"entry wrap\"><nav class=\"crumbs\" aria-label=\"Breadcrumb\"><a href=\"{}\">{}</a><span aria-hidden=\"true\">/</span><a href=\"{}#q-{}\">{}</a></nav>",
        ctx.href(&from, "radar/"),
        esc(m.curated()),
        ctx.href(&from, "radar/"),
        m.site.quadrants[qi].id,
        esc(m.quad_name(qi))
    );
    b.push_str("<header class=\"entry-head\"><div class=\"entry-id\">");
    // The breadcrumb names the quadrant, so the eyebrow names only the maker.
    if let Some(x) = e.maker.as_deref() {
        let _ = write!(b, "<p class=\"eyebrow\"><span>{}</span></p>", esc(x));
    }
    // A long name takes a smaller size, so the title stays within three lines and the verdict
    // stays near the top.
    let long = if e.name.chars().count() > 24 { " class=\"long\"" } else { "" };
    let _ = write!(b, "<h1{long}>{}</h1><p class=\"lede\">{}</p>", esc(&e.name), inline(&e.summary));
    let mut desc = e.summary.clone();
    if let Some(i) = placed {
        let bv = &main.blips[i];
        let last = main.latest();
        b.push_str("<div class=\"verdict\">");
        match bv.ring_now() {
            Some(k) => {
                b.push_str(&ring_chip(ctx, k));
                desc = format!("{} on {}. {}", m.ring_name(k), m.curated(), e.summary);
            }
            None => b.push_str("<span class=\"ring retired\">Retired</span>"),
        }
        let lc = bv.last_change();
        let st = bv.state_at(lc);
        let since = if lc == 0 && st == State::New { format!("On the radar since {}", main.dates[0].human()) } else { format!("{} on {}", st.label(), main.dates[lc].human()) };
        // Edition 01 is the starting point: its entries wear no "new" mark.
        let _ = write!(b, "<span class=\"since\">{}{}</span>", if lc == last && last > 0 { mark(st) } else { String::new() }, esc(&since));
        b.push_str("</div>");
    } else {
        let _ = write!(b, "<div class=\"verdict\"><span class=\"since\">Not on {}</span></div>", esc(m.curated()));
    }
    if let Some(u) = &e.url {
        let host = u.trim_start_matches("https://").split('/').next().unwrap_or(u).trim_start_matches("www.");
        let _ = write!(b, "<p class=\"visit\"><a href=\"{}\" rel=\"noopener\">{} <span aria-hidden=\"true\">→</span></a></p>", esc(u), esc(host));
    }
    b.push_str("</div>");
    if let Some(i) = placed {
        let _ = write!(b, "<figure class=\"entry-scope\">{}<figcaption class=\"label\">Position on the radar</figcaption></figure>", svg::quadrant(ctx, main, i));
    }
    b.push_str("</header>");

    if let Some(i) = placed {
        let bv = &main.blips[i];
        // The verdict is written in the first person: the heading names who speaks.
        let heading = match bv.ring_now() {
            Some(k) => format!("{}'s verdict: {}", m.curator(), m.ring_name(k)),
            None => format!("Why {} retired it", m.curator()),
        };
        // A retirement rewrites the rationale (apply-edition.py), so the article explains the retirement.
        let _ = write!(b, "<section class=\"sect why\"><h2>{}</h2><div class=\"sect-body\"><div class=\"prose\">{}</div></div></section>", esc(&heading), paras(&bv.b.rationale));

        b.push_str("<section class=\"sect movement\"><h2>What moved</h2><div class=\"sect-body\">");
        // The ring chart starts with the second edition that places the entry: one dot says nothing the list does not.
        if (0..main.dates.len()).filter(|&x| bv.rings[x].is_some()).count() > 1 {
            b.push_str(&svg::track(ctx, main, i));
        }
        b.push_str("<ol class=\"timeline\">");
        for (hi, h) in bv.b.history.iter().enumerate().rev() {
            let e_ix = main.dates.iter().position(|d| *d == h.date).unwrap();
            // Edition 01 is the baseline: what it placed wears no "new" mark.
            let st = match bv.state_at(e_ix) {
                State::New if e_ix == 0 => State::Steady,
                st => st,
            };
            let what = match h.ring.as_str() {
                crate::model::RETIRED => "Retired".to_string(),
                r => {
                    let k = m.site.rings.iter().position(|x| x.id == r).unwrap();
                    if hi == 0 { format!("Added in {}", m.ring_name(k)) } else { format!("Moved to {}", m.ring_name(k)) }
                }
            };
            let _ = write!(
                b,
                "<li class=\"{}\">{}<a class=\"tl-date\" href=\"{}\"><time datetime=\"{}\">{}</time></a><span class=\"tl-what\">{}</span><span class=\"tl-note\">{}</span></li>",
                st.class(),
                mark(st),
                ctx.href(&from, &format!("moves/{}/", h.date)),
                h.date,
                h.date.human(),
                esc(&what),
                inline(&h.note)
            );
        }
        b.push_str("</ol></div></section>");
    }

    // Other radars: where other people place the same entry. Example radars only demonstrate the
    // comparison on their own page, so they never count as a placement here.
    let others: Vec<(&RadarView, usize)> =
        m.radars.iter().skip(1).filter(|rv| !rv.r.example).filter_map(|rv| rv.find(&e.id).and_then(|bv| bv.ring_now()).map(|k| (rv, k))).collect();
    b.push_str("<section class=\"sect others\"><h2>On other radars</h2><div class=\"sect-body\">");
    if others.is_empty() {
        b.push_str("<p class=\"quiet\">No community radar places this yet.</p>");
    } else {
        b.push_str("<ul class=\"placements\">");
        for (rv, k) in &others {
            let _ = write!(
                b,
                "<li><a href=\"{}\">{}</a> <span class=\"by\">by {}</span>{}</li>",
                ctx.href(&from, &format!("r/{}/", rv.r.id)),
                esc(&rv.r.title),
                esc(&rv.r.owner),
                ring_chip(ctx, *k)
            );
        }
        b.push_str("</ul>");
    }
    // The builder opens at this entry's row (build.js reads `#e=`).
    let _ =
        write!(b, "<p><a class=\"btn ghost\" href=\"{}#e={}\">Place it on your radar <span aria-hidden=\"true\">→</span></a></p></div></section>", ctx.href(&from, "build/"), e.id);

    b.push_str("<section class=\"sect sources\"><h2>Sources</h2><div class=\"sect-body\">");
    if e.evidence.is_empty() {
        let none = if placed.is_some() { format!("No outside source. The verdict comes from {}'s own work.", m.curator()) } else { "No sources recorded yet.".into() };
        let _ = write!(b, "<p class=\"quiet\">{}</p>", esc(&none));
    } else {
        b.push_str("<ul>");
        for ev in &e.evidence {
            let _ = write!(b, "<li><a href=\"{}\" rel=\"noopener\">{}</a> <span class=\"by\">checked {}</span></li>", esc(&ev.url), inline(&ev.claim), ev.checked_on.human());
        }
        b.push_str("</ul>");
    }
    b.push_str("</div></section>");

    // Share and embed: the growth loop for makers.
    let url = ctx.abs(&from);
    let badge = ctx.abs(&format!("badges/{}.svg", e.id));
    let md = format!("[![{} on Product Radar]({badge})]({url})", e.name);
    let html = format!("<a href=\"{url}\"><img src=\"{badge}\" alt=\"{} on Product Radar\" height=\"28\"></a>", esc(&e.name));
    let text = match placed.and_then(|i| main.blips[i].ring_now()) {
        Some(k) => format!("{}: {} on {}", e.name, m.ring_name(k), m.curated()),
        None => format!("{} on {}", e.name, m.site.title),
    };
    let enc = |s: &str| pct(s);
    let _ = write!(
        b,
        "<section class=\"sect share\"><h2>Share and embed</h2><div class=\"sect-body\"><div class=\"share-grid\"><div><p>Makers: show where your product sits. The badge updates when the radar does.</p><p class=\"badge-prev\"><img src=\"{bsrc}\" alt=\"Badge for {name_e}\" width=\"236\" height=\"28\"></p>\
<div class=\"snip\"><label for=\"s-md\">Markdown</label><textarea id=\"s-md\" readonly rows=\"4\">{md}</textarea><button type=\"button\" class=\"copy\" data-copy=\"s-md\">Copy</button></div>\
<div class=\"snip\"><label for=\"s-html\">HTML</label><textarea id=\"s-html\" readonly rows=\"6\">{html}</textarea><button type=\"button\" class=\"copy\" data-copy=\"s-html\">Copy</button></div></div>\
<div><p>Share this entry</p><ul class=\"share-links\"><li><a href=\"https://x.com/intent/post?text={t}&amp;url={u}\" rel=\"noopener\">X</a></li><li><a href=\"https://www.linkedin.com/sharing/share-offsite/?url={u}\" rel=\"noopener\">LinkedIn</a></li><li><a href=\"https://bsky.app/intent/compose?text={tu}\" rel=\"noopener\">Bluesky</a></li><li><button type=\"button\" class=\"copy\" data-copy-text=\"{url_e}\">Copy link</button></li></ul></div></div></div></section>",
        bsrc = ctx.href(&from, &format!("badges/{}.svg", e.id)),
        md = esc(&md),
        html = esc(&html),
        t = enc(&text),
        u = enc(&url),
        tu = enc(&format!("{text} {url}")),
        url_e = esc(&url),
        name_e = esc(&e.name),
    );

    // Neighbours in the same quadrant and ring.
    if let Some(i) = placed {
        let bv = &main.blips[i];
        if let Some(k) = bv.ring_now() {
            let n: Vec<_> = main.on_radar(main.latest()).filter(|(j, x)| *j != i && x.q == bv.q && x.rings[main.latest()] == Some(k)).collect();
            if !n.is_empty() {
                let _ = write!(b, "<section class=\"sect near\"><h2>Also in {}\u{a0}· {}</h2><div class=\"sect-body\"><ul>", esc(m.quad_name(qi)), esc(m.ring_name(k)));
                for (_, x) in n {
                    let _ = write!(b, "<li><a href=\"{}\">{}</a><span>{}</span></li>", ctx.href(&from, &format!("e/{}/", x.e.id)), esc(&x.e.name), inline(&x.e.summary));
                }
                b.push_str("</ul></div></section>");
            }
        }
    }
    b.push_str("</article>");
    Page {
        path: from.clone(),
        title: e.name.clone(),
        desc,
        og: format!("og/e/{}.png", e.id),
        nav: "radar",
        class: "p-entry",
        body: b,
        scripts: &["site.js"],
        head_extra: String::new(),
    }
}

pub fn pct(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            o.push(b as char);
        } else {
            let _ = write!(o, "%{:02X}", b);
        }
    }
    o
}

fn edition_article(ctx: &Ctx, from: &str, rv: &RadarView, e: usize, heading: &str) -> String {
    let d = rv.dates[e];
    let ed = rv.edition(e);
    format!(
        "<article class=\"ed\" id=\"ed-{d}\"><header class=\"ed-stamp\"><a href=\"{href}\"><span class=\"ed-n\">Edition</span><span class=\"ed-w\">{n:02}</span><time datetime=\"{d}\">{hd}</time></a></header><div class=\"ed-body\"><{heading}>{t}</{heading}><p class=\"ed-note\">{note}</p>{body}{list}</div></article>",
        href = ctx.href(from, &format!("moves/{d}/")),
        n = e + 1,
        hd = d.human(),
        t = esc(&ed.title),
        note = inline(&ed.note),
        // The article shows on the edition's own page only; /moves/ lists the notes.
        body = match &ed.body {
            Some(t) if heading == "h1" && !t.trim().is_empty() => format!("<div class=\"ed-article prose\">{}</div>", paras(t)),
            _ => String::new(),
        },
        list = if e == 0 && heading == "h1" {
            // The edition's own page: every entry it placed, ring by ring, with the note on why.
            format!("{}{}", baseline_sum(ctx, rv), baseline_list(ctx, from, rv))
        } else if e == 0 {
            format!(
                "{}<p>Edition 01 is the starting point, so it lists no moves. From edition 02 on, each edition lists its moves and the reason for each.</p><p><a class=\"more\" href=\"{}\">See the {} entries it placed</a></p>",
                baseline_sum(ctx, rv),
                ctx.href(from, &format!("moves/{d}/")),
                rv.on_radar(0).count()
            )
        } else {
            moves_list(ctx, from, rv, e, true, usize::MAX)
        },
    )
}

/// Edition 01 placed every entry: one group per ring, each entry with its quadrant and the note on why.
fn baseline_list(ctx: &Ctx, from: &str, rv: &RadarView) -> String {
    let mut o = String::from("<div class=\"ed-rings\">");
    for k in 0..4 {
        let items: Vec<&BlipView> = rv.on_radar(0).map(|(_, b)| b).filter(|b| b.rings[0] == Some(k)).collect();
        if items.is_empty() {
            continue;
        }
        let _ = write!(o, "<section class=\"ringrow r{k}\"><h2>{}<span class=\"count\">{}</span></h2><ul class=\"ed-list\">", ring_chip(ctx, k), items.len());
        for b in items {
            let note = b.b.history.first().map(|h| h.note.as_str()).unwrap_or("");
            let _ = write!(
                o,
                "<li><span><a href=\"{}\">{}</a><span class=\"by\">{}</span></span><span>{}</span></li>",
                ctx.href(from, &format!("e/{}/", b.e.id)),
                esc(&b.e.name),
                esc(ctx.m.quad_name(b.q)),
                inline(note)
            );
        }
        o.push_str("</ul></section>");
    }
    o.push_str("</div>");
    o
}

pub fn moves(ctx: &Ctx) -> Page {
    let rv = ctx.m.main();
    let from = "moves/";
    let mut b = format!(
        "<section class=\"page-head wrap\"><h1>What moved</h1><p class=\"lede\">Every change to {}, edition by edition, with the reason for each move. <a href=\"{}\">Follow the Atom feed</a>.</p></section><div class=\"wrap editions\">",
        esc(ctx.m.curated()),
        ctx.href(from, "feed.xml")
    );
    for e in (0..rv.dates.len()).rev() {
        b.push_str(&edition_article(ctx, from, rv, e, "h2"));
    }
    b.push_str("</div>");
    Page {
        path: from.into(),
        title: "What moved".into(),
        desc: format!("Every change to {}, edition by edition, with the reason for each move.", ctx.m.curated()),
        og: "og/moves.png".into(),
        nav: "moves",
        class: "p-moves",
        body: b,
        scripts: &["site.js"],
        head_extra: String::new(),
    }
}

pub fn edition(ctx: &Ctx, e: usize) -> Page {
    let rv = ctx.m.main();
    let d = rv.dates[e];
    let from = format!("moves/{d}/");
    let ed = rv.edition(e);
    let mut b = format!(
        "<section class=\"page-head wrap\"><nav class=\"crumbs\" aria-label=\"Breadcrumb\"><a href=\"{}\">What moved</a><span aria-hidden=\"true\">/</span><span>Edition {:02}</span></nav></section><div class=\"wrap editions single\">",
        ctx.href(&from, "moves/"),
        e + 1
    );
    b.push_str(&edition_article(ctx, &from, rv, e, "h1"));
    b.push_str("<nav class=\"ed-nav\" aria-label=\"Editions\">");
    if e > 0 {
        let _ = write!(b, "<a rel=\"prev\" href=\"{}\"><span aria-hidden=\"true\">←</span> Edition {:02}</a>", ctx.href(&from, &format!("moves/{}/", rv.dates[e - 1])), e);
    }
    if e < rv.latest() {
        let _ = write!(b, "<a rel=\"next\" href=\"{}\">Edition {:02} <span aria-hidden=\"true\">→</span></a>", ctx.href(&from, &format!("moves/{}/", rv.dates[e + 1])), e + 2);
    }
    b.push_str("</nav></div>");
    let desc = if e == 0 {
        let n = ring_counts(rv, 0, None);
        let rings: Vec<String> = n.iter().enumerate().map(|(k, c)| format!("{c} in {}", ctx.m.ring_name(k))).collect();
        format!("{} Edition 01 of {}, {}: {} entries. {}.", ed.note, ctx.m.curated(), d.human(), rv.on_radar(0).count(), rings.join(", "))
    } else {
        let mvs = rv.moves_at(e);
        let names: Vec<&str> = mvs.iter().take(4).map(|mv| rv.blips[mv.blip].e.name.as_str()).collect();
        format!("{} What moved on {} in the week of {}: {}.", ed.note, ctx.m.curated(), d.human(), names.join(", "))
    };
    Page {
        path: from,
        title: format!("Edition {:02}: {}", e + 1, ed.title),
        desc,
        og: format!("og/moves/{d}.png"),
        nav: "moves",
        class: "p-moves",
        body: b,
        scripts: &["site.js"],
        head_extra: String::new(),
    }
}

pub fn about(ctx: &Ctx) -> Page {
    let m = ctx.m;
    let from = "about/";
    let mut b = String::from(
        "<section class=\"page-head wrap\"><h1>About Product Radar</h1><p class=\"lede\">Why the radar exists and how Konstantin decides what goes on it.</p></section>",
    );
    b.push_str(
        "<section class=\"sect wrap\" id=\"why\" aria-labelledby=\"why-h\"><h2 id=\"why-h\">Why it exists</h2><div class=\"sect-body\"><div class=\"cols\">\
<div><p>With good AI agents, one person can now take a customer problem all the way to a working business. Until recently, that took a team.</p>\
<p>Agents now write much of the code. The harder work is to find a problem worth solving and to build a sustainable business around it.</p></div>\
<div><p>The tools for this work change every week. Most coverage of them is hype, or it targets specialist engineers.</p>\
<p>On Product Radar, Konstantin Kaufmann shows which tools he uses to build and run his own products, and which ones he dropped. Each verdict says why.</p></div></div></div></section>",
    );
    b.push_str(
        "<section class=\"sect wrap\" id=\"method\" aria-labelledby=\"method-h\"><h2 id=\"method-h\">What goes on the radar</h2><div class=\"sect-body\"><div class=\"cols\">\
<div><p>An entry goes on the radar only if it changes what one person with AI agents can build or run. A new launch alone does not qualify.</p>\
<p>More tools launch every week than anyone can track, so most candidates stay off the radar.</p>\
<p>Each verdict comes from Konstantin's own work on his products. Your own radar can disagree with his, and the builder shows you where.</p></div>\
<div><h3>The weekly edition</h3><ol class=\"steps\"><li>During the week, Konstantin tries tools on real work with his agents and decides what stays.</li><li>At the end of the week, an agent reads his working sessions. It drafts each move with his reason and a short article.</li><li>Independent reviewer agents read the draft as a visitor would. They check that it is clear and sourced. They also check for private details.</li><li>When every check passes, the edition ships with its own page, social card and feed entry. Konstantin does not review it first.</li></ol></div></div></div></section>",
    );
    let mut legend = String::new();
    for (st, text) in [
        (State::New, "Added in this edition."),
        (State::In, "Moved towards the centre: a stronger recommendation."),
        (State::Out, "Moved towards Hold: a weaker recommendation."),
        (State::Retired, "Left the radar in this edition. Listed under What moved."),
        (State::Steady, "Same ring as in the edition before."),
    ] {
        let _ = write!(legend, "<li class=\"{}\"><b>{}{}</b><span>{}</span></li>", st.class(), mark(st), st.label(), text);
    }
    let _ = write!(
        b,
        "<section class=\"sect wrap\" id=\"movement\" aria-labelledby=\"movement-h\"><h2 id=\"movement-h\">How the radar shows movement</h2><div class=\"sect-body\"><div class=\"cols\">\
<div><p>Strong colour on a dot shows change. Entries that did not move stay plain ink, so your eye goes to what moved first.</p>\
<p>The rings carry soft tints: teal is strongest at Adopt and fades towards Hold.</p>\
<p>A moved entry draws a trail back to the ring it came from, like the target history on a radar screen. Older trails fade.</p>\
<p>Every state also has its own shape, so the radar reads without colour too.</p>\
<p>On community radars, the same colours show disagreement with {}: rated higher, rated lower, or only on that radar.</p></div>\
<div><ul class=\"legend terms\">{legend}</ul></div></div></div></section>",
        esc(m.curated())
    );
    let main = m.main();
    if !main.r.rejected.is_empty() {
        b.push_str("<section class=\"sect wrap\" id=\"rejected\" aria-labelledby=\"rejected-h\"><h2 id=\"rejected-h\">Not on the radar</h2><div class=\"sect-body\"><p class=\"sect-lede\">Considered and left off, with the reason.</p><ul class=\"rejected\">");
        for rj in &main.r.rejected {
            let name = match &rj.url {
                Some(u) => format!("<a href=\"{}\" rel=\"noopener\">{}</a>", esc(u), esc(&rj.name)),
                None => esc(&rj.name),
            };
            let _ = write!(b, "<li><b>{name}</b><span>{}</span><time datetime=\"{}\">{}</time></li>", inline(&rj.reason), rj.date, rj.date.human());
        }
        b.push_str("</ul></div></section>");
    }
    let _ = write!(
        b,
        "<section class=\"sect wrap\" id=\"colophon\" aria-labelledby=\"colophon-h\"><h2 id=\"colophon-h\">How Konstantin builds this site</h2><div class=\"sect-body\">\
<p class=\"sect-lede\">Agents write most of this site, so Konstantin chose its stack for code that agents write and check. A program checks the data before it publishes. No person proofreads each page.</p><dl class=\"dl3\">\
<div><dt>Generator</dt><dd>A small program in Rust, a programming language whose compiler catches many mistakes. The compiler and two more checks stop agent mistakes before a build ships. One check finds broken data. The other finds private details. This is <a href=\"{}\">Guardrails, not manual review</a>.</dd></div>\
<div><dt>Data</dt><dd>Plain text files in TOML, a simple data format: one per entry and one per radar.</dd></div>\
<div><dt>Hosting</dt><dd>Plain web pages (static HTML) with a little JavaScript, on <a href=\"{}\">Cloudflare Pages</a>.</dd></div>\
<div><dt>Agents</dt><dd>Built with <a href=\"{}\">Claude Code</a>.</dd></div>\
<div><dt>Type</dt><dd>Archivo and Martian Mono.</dd></div></dl></div></section>",
        ctx.href(from, "e/guardrails-not-manual-review/"),
        ctx.href(from, "e/cloudflare-pages/"),
        ctx.href(from, "e/claude-code/")
    );
    Page {
        path: from.into(),
        title: "About".into(),
        desc: "Why Product Radar exists and how Konstantin Kaufmann decides what goes on his radar.".into(),
        og: "og/about.png".into(),
        nav: "about",
        class: "p-about",
        body: b,
        scripts: &["site.js"],
        head_extra: String::new(),
    }
}

pub fn community(ctx: &Ctx, ri: usize) -> Page {
    let m = ctx.m;
    let rv = &m.radars[ri];
    let main = m.main();
    let from = format!("r/{}/", rv.r.id);
    let e = rv.latest();
    let ex = if rv.r.example { "<span>Example</span>" } else { "" };
    let mut b = format!(
        "<section class=\"page-head wrap\"><p class=\"eyebrow\"><span>{}</span>{ex}<span>{}</span></p><h1>{}</h1><p class=\"lede\">{} <span class=\"by\">By {}.</span></p></section>",
        if rv.r.kind == Kind::Community { "Community radar" } else { "Radar" },
        rv.dates[e].human(),
        esc(&rv.r.title),
        inline(&rv.r.description),
        esc(&rv.r.owner)
    );
    // Comparison with the curated radar.
    let me = main.latest();
    let mut same = 0;
    let mut rows = Vec::new();
    for (_, bv) in rv.on_radar(e) {
        let k = bv.rings[e].unwrap();
        let ours = main.find(&bv.e.id).and_then(|x| x.rings[me]);
        if ours == Some(k) {
            same += 1;
        }
        rows.push((bv, k, ours));
    }
    let shared = rows.iter().filter(|r| r.2.is_some()).count();
    rows.sort_by_key(|r| (std::cmp::Reverse(r.2.map(|o| (o as i32 - r.1 as i32).abs()).unwrap_or(-1)), r.0.e.name.to_lowercase()));
    let mut side = format!(
        "<aside class=\"log compare-sum\"><h2 class=\"label\">Compared with {}</h2><p class=\"big\"><b>{same}</b> of {shared}</p><p>shared entries sit in the same ring. The biggest disagreements come first.</p><ol class=\"moves\">",
        esc(m.curated())
    );
    for (bv, k, ours) in rows.iter().filter(|r| r.2.is_some_and(|o| o != r.1)).take(6) {
        let o = ours.unwrap();
        let st = if *k < o { State::In } else { State::Out };
        let _ = write!(
            side,
            "<li class=\"mv {}\">{}<span class=\"mv-main\"><a href=\"{}\">{}</a><span class=\"mv-rings\">Theirs: {} · {}: {}</span></span></li>",
            st.class(),
            mark(st),
            ctx.href(&from, &format!("e/{}/", bv.e.id)),
            esc(&bv.e.name),
            esc(m.ring_name(*k)),
            esc(m.curator()),
            esc(m.ring_name(o))
        );
    }
    let _ = write!(
        side,
        "</ol><ul class=\"legend small\"><li>{} They rate it higher</li><li>{} They rate it lower</li><li>{} Only on their radar</li><li>{} Same ring</li></ul><a class=\"more\" href=\"{}\">Build your own <span aria-hidden=\"true\">→</span></a></aside>",
        mark(State::In),
        mark(State::Out),
        mark(State::New),
        mark(State::Steady),
        ctx.href(&from, "build/")
    );
    b.push_str(&deck(ctx, &from, rv, &side));
    // Side by side: the differences first, as a table across the full width. The entries in the
    // same ring follow as short lists, one per ring, so the long run of matching rows collapses.
    let differ: Vec<_> = rows.iter().filter(|r| r.2 != Some(r.1)).collect();
    let _ = write!(b, "<section class=\"wrap compare\"><h2>Side by side</h2><h3 class=\"cmp-h\">Where they differ<span class=\"count\">{}</span></h3>", differ.len());
    if differ.is_empty() {
        b.push_str("<p class=\"quiet\">Every entry on this radar sits in the same ring as on ");
        b.push_str(&esc(m.curated()));
        b.push_str(".</p>");
    } else {
        let _ = write!(
            b,
            "<div class=\"tbl\"><table class=\"side\"><thead><tr><th scope=\"col\">Entry</th><th scope=\"col\" class=\"c-q\">Quadrant</th><th scope=\"col\">Theirs</th><th scope=\"col\">{}</th><th scope=\"col\">Difference</th></tr></thead><tbody>",
            esc(m.curator())
        );
        for (bv, k, ours) in differ {
            let (cls, o, what) = match ours {
                Some(o) => {
                    let d = (*o as i32 - *k as i32).unsigned_abs();
                    let n = if d == 1 { "1 ring".to_string() } else { format!("{d} rings") };
                    if o > k {
                        (" class=\"diff d-in\"", ring_chip(ctx, *o), format!("{} Higher by {n}", mark(State::In)))
                    } else {
                        (" class=\"diff d-out\"", ring_chip(ctx, *o), format!("{} Lower by {n}", mark(State::Out)))
                    }
                }
                None => ("", "<span class=\"quiet\">Not placed</span>".to_string(), format!("{} Only on their radar", mark(State::New))),
            };
            let _ = write!(
                b,
                "<tr{cls}><th scope=\"row\"><a href=\"{}\">{}</a></th><td class=\"c-q\">{}</td><td>{}</td><td>{o}</td><td class=\"c-d\">{what}</td></tr>",
                ctx.href(&from, &format!("e/{}/", bv.e.id)),
                esc(&bv.e.name),
                esc(&m.site.quadrants[bv.q].name),
                ring_chip(ctx, *k)
            );
        }
        b.push_str("</tbody></table></div>");
    }
    let alike: Vec<_> = rows.iter().filter(|r| r.2 == Some(r.1)).collect();
    if !alike.is_empty() {
        let _ = write!(b, "<h3 class=\"cmp-h\">In the same ring<span class=\"count\">{}</span></h3><div class=\"same-rings\">", alike.len());
        for k in 0..m.site.rings.len() {
            let mut names: Vec<_> = alike.iter().filter(|r| r.1 == k).map(|r| r.0).collect();
            if names.is_empty() {
                continue;
            }
            names.sort_by_key(|bv| bv.e.name.to_lowercase());
            let _ = write!(b, "<div class=\"ringrow r{k}\"><h4>{}</h4><ul>", ring_chip(ctx, k));
            for bv in names {
                let _ = write!(b, "<li><a href=\"{}\">{}</a></li>", ctx.href(&from, &format!("e/{}/", bv.e.id)), esc(&bv.e.name));
            }
            b.push_str("</ul></div>");
        }
        b.push_str("</div>");
    }
    b.push_str("</section>");
    Page {
        path: from.clone(),
        title: rv.r.title.clone(),
        desc: format!("{} A community radar by {}, compared with {}.", rv.r.description, rv.r.owner, m.curated()),
        og: format!("og/r/{}.png", rv.r.id),
        nav: "build",
        class: "p-radar",
        body: b,
        scripts: &["site.js"],
        head_extra: if rv.r.example { "<meta name=\"robots\" content=\"noindex\">\n".into() } else { String::new() },
    }
}

pub fn build(ctx: &Ctx) -> Page {
    let m = ctx.m;
    let main = m.main();
    let from = "build/";
    let e = main.latest();
    // Catalog: every entry, with the curated ring and the blip's place on the curated scope
    // when placed, and the short scope label when there is one.
    let corners: Vec<crate::model::Corner> = m.site.quadrants.iter().map(|q| q.position).collect();
    let curated_pos = layout::layout(main, &corners, e);
    // The curated scope's labels: a blip that keeps its curated ring keeps its label too, so a
    // shared radar names every blip the way Konstantin's radar does (build.js `fixedLabels`).
    let curated_labels = svg::overview_labels(ctx, main);
    let mut cat = String::from("[");
    for (i, en) in m.entries.iter().enumerate() {
        let q = m.site.quadrants.iter().position(|q| q.id == en.quadrant).unwrap();
        let ours = main.find(&en.id).and_then(|b| b.rings[e]).map(|k| k as i32).unwrap_or(-1);
        let short = en.label.as_deref().map(|l| format!(",\"l\":{}", json_str(l))).unwrap_or_default();
        let bi = main.blips.iter().position(|b| b.e.id == en.id);
        let at = bi.and_then(|bi| curated_pos[bi]).map(|p| format!(",\"p\":[{:.1},{:.1}]", p.x, p.y)).unwrap_or_default();
        let lab = bi
            .and_then(|bi| curated_labels.iter().find(|l| l.blip == bi))
            .map(|l| {
                let lines = l.lines.iter().map(|x| json_str(x)).collect::<Vec<_>>().join(",");
                let lead = l.lead.map(|s| format!("[{:.1},{:.1},{:.1},{:.1}]", s[0], s[1], s[2], s[3])).unwrap_or_else(|| "0".into());
                format!(",\"L\":[{:.1},{:.1},\"{}\",[{lines}],{lead}]", l.x, l.y, l.anchor)
            })
            .unwrap_or_default();
        let _ = write!(cat, "{}{{\"id\":{},\"n\":{},\"q\":{q},\"o\":{ours}{short}{at}{lab}}}", if i > 0 { "," } else { "" }, json_str(&en.id), json_str(&en.name));
    }
    cat.push(']');
    let cfg = format!(
        "{{\"cat\":{cat},\"q\":[{}],\"c\":[{}],\"r\":[{}],\"ri\":[{}],\"ro\":[{}],\"fo\":{},\"base\":{},\"entry\":{},\"name\":{},\"who\":{}}}",
        m.site.quadrants.iter().map(|q| json_str(&q.name)).collect::<Vec<_>>().join(","),
        m.site.quadrants.iter().map(|q| format!("\"{}\"", corner_code(q.position))).collect::<Vec<_>>().join(","),
        m.site.rings.iter().map(|r| json_str(&r.name)).collect::<Vec<_>>().join(","),
        m.site.rings.iter().map(|r| json_str(&r.id)).collect::<Vec<_>>().join(","),
        layout::RING_OUTER.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(","),
        layout::F_OVER,
        json_str(&ctx.abs("build/")),
        json_str(&ctx.href(from, "e/")),
        json_str(m.curated()),
        json_str(m.curator()),
    );
    let mut list = String::new();
    let who = esc(m.curator());
    for (q, quad) in m.site.quadrants.iter().enumerate() {
        let mut es: Vec<_> = m.entries.iter().filter(|x| x.quadrant == quad.id).collect();
        es.sort_by_key(|x| x.name.to_lowercase());
        let _ = write!(list, "<section class=\"b-q\" data-q=\"{q}\"><h2><span>{}</span><span class=\"count\">{}</span></h2><ul>", esc(&quad.name), es.len());
        for en in es {
            // The curated ring: named in words under the entry, outlined in the control.
            let ours = main.find(&en.id).and_then(|b| b.rings[e]);
            // When the visitor's ring differs, the curated ring word and its segment both get a dashed mark.
            let r#ref = match ours {
                Some(k) => format!("<span class=\"b-ref\"><span>{who}: <span class=\"b-o\">{}</span></span><span class=\"b-diff\"></span></span>", esc(m.ring_name(k))),
                None => format!("<span class=\"b-ref none\"><span>Not on {who}'s radar</span><span class=\"b-diff\"></span></span>"),
            };
            let _ = write!(
                list,
                "<li data-id=\"{}\"><span class=\"b-name\"><span class=\"b-n\">{}</span>{}</span><span class=\"seg\" role=\"radiogroup\" aria-label=\"Ring for {}\">",
                en.id,
                esc(&en.name),
                r#ref,
                esc(&en.name)
            );
            // The markup starts checked at the curated ring, as most visits do, so nothing flips on load.
            for (k, r) in m.site.rings.iter().enumerate() {
                let (cls, sr) = if ours == Some(k) { (" ours", format!("<span class=\"sr\">, {who}'s ring</span>")) } else { ("", String::new()) };
                let on = ours == Some(k);
                let _ = write!(
                    list,
                    "<button type=\"button\" role=\"radio\" aria-checked=\"{on}\" tabindex=\"{}\" data-k=\"{k}\" class=\"r{k}{cls}\">{}<span>{}</span>{sr}</button>",
                    if on { 0 } else { -1 },
                    meter(k),
                    esc(&r.name)
                );
            }
            let none = ours.is_none();
            let _ = write!(
                list,
                "<button type=\"button\" role=\"radio\" aria-checked=\"{none}\" tabindex=\"{}\" data-k=\"-1\" class=\"none\">Not on my radar</button></span></li>",
                if none { 0 } else { -1 }
            );
        }
        list.push_str("</ul></section>");
    }
    let mut empty_scope = String::new();
    let _ = write!(empty_scope, "<svg class=\"radar\" viewBox=\"{a} {a} {w} {w}\" aria-hidden=\"true\" focusable=\"false\"><g class=\"z\">", a = -layout::VB, w = 2.0 * layout::VB);
    // Reuse the full furniture by rendering a scope of zero blips: cheap and consistent.
    let full = svg::scope(ctx, main, from, false);
    let furniture_end = full.find("<g class=\"trails\">").unwrap_or(full.len());
    let furniture_start = full.find("<g class=\"z\">").map(|i| i + 13).unwrap_or(0);
    empty_scope.push_str(&full[furniture_start..furniture_end]);
    empty_scope.push_str("<g class=\"trails\"></g><g class=\"blips\"></g><g class=\"lbl lo\"></g></g></svg>");

    let body = format!(
        "<section class=\"page-head wrap\"><p class=\"eyebrow\"><span>Your radar</span></p><h1>Build your radar</h1><p class=\"lede\">The list starts with {curated}. Move what you disagree with, or clear the list and start blank. Your radar lives in its share link, so you need no account and no server stores it.</p></section>\
<div class=\"builder wrap\" data-builder>\
<div class=\"b-main\"><div class=\"b-tools\"><label class=\"sr\" for=\"b-filter\">Filter entries</label><input type=\"search\" id=\"b-filter\" placeholder=\"Filter entries\" autocomplete=\"off\"><button type=\"button\" class=\"btn ghost\" id=\"b-seed\">Reset to {curated}</button><button type=\"button\" class=\"btn ghost\" id=\"b-clear\">Clear all</button></div>\
<div class=\"b-list\">{list}</div>\
<form class=\"b-add\" id=\"b-add\"><h2 class=\"label\">Add something missing</h2><div class=\"b-add-row\"><label class=\"sr\" for=\"b-add-name\">Name</label><input id=\"b-add-name\" required maxlength=\"40\" placeholder=\"Name\"><label class=\"sr\" for=\"b-add-q\">Quadrant</label><select id=\"b-add-q\">{qopts}</select><label class=\"sr\" for=\"b-add-r\">Ring</label><select id=\"b-add-r\">{ropts}</select><button class=\"btn\" type=\"submit\">Add</button></div></form>\
<div class=\"b-bar\"><p class=\"b-bar-n\" aria-hidden=\"true\"></p><p class=\"b-msg\" role=\"status\"></p><a class=\"btn ghost\" href=\"#b-card\">See your radar</a><button type=\"button\" class=\"btn\" data-share>Copy share link</button></div>\
</div>\
<aside class=\"b-side\"><div class=\"b-card\" id=\"b-card\"><label class=\"label\" for=\"b-owner\">Whose radar</label><input id=\"b-owner\" maxlength=\"40\" placeholder=\"Your name or handle\" autocomplete=\"nickname\">\
<div class=\"b-actions\"><button type=\"button\" class=\"btn\" data-share>Copy share link</button><button type=\"button\" class=\"btn ghost\" id=\"b-png\">Download image</button></div>\
<p class=\"b-msg\" role=\"status\"></p>\
<figure class=\"scope b-scope\">{empty_scope}</figure>\
<div class=\"b-stats\" aria-live=\"polite\"></div>\
<h2 class=\"label b-dh\" hidden>Biggest disagreements</h2><ol class=\"moves b-dl\"></ol><p class=\"b-left\" hidden></p>\
<details class=\"b-toml\"><summary>Export as a radar file</summary><p>Save it as <code>data/radars/your-name.toml</code> and open a pull request to publish it on the site.</p><textarea id=\"b-toml\" readonly rows=\"8\"></textarea><button type=\"button\" class=\"copy\" data-copy=\"b-toml\">Copy</button></details>\
</div></aside></div>\
<noscript><p class=\"wrap quiet\">The builder needs JavaScript. The radar itself works without it.</p></noscript>\
<script type=\"application/json\" id=\"build-data\">{cfg}</script>",
        curated = esc(m.curated()),
        qopts = m.site.quadrants.iter().enumerate().map(|(q, x)| format!("<option value=\"{q}\">{}</option>", esc(&x.name))).collect::<String>(),
        ropts = m.site.rings.iter().enumerate().map(|(k, x)| format!("<option value=\"{k}\">{}</option>", esc(&x.name))).collect::<String>(),
    );
    Page {
        path: from.into(),
        title: "Build your radar".into(),
        desc: format!("Place the tools you use in your own rings, share the link and see where you disagree with {}. You need no account.", m.curated()),
        og: "og/build.png".into(),
        nav: "build",
        class: "p-build",
        body,
        scripts: &["site.js", "build.js"],
        head_extra: String::new(),
    }
}

pub fn not_found(ctx: &Ctx) -> Page {
    let body = format!(
        "<section class=\"page-head wrap nf\"><p class=\"eyebrow\"><span>404</span></p><h1>Page not found</h1><p class=\"lede\">The address may be wrong, or the page has moved. <a href=\"{}\">Open {}</a> or <a href=\"{}\">see what moved</a>.</p></section>",
        "/radar/",
        esc(ctx.m.curated()),
        "/moves/"
    );
    Page {
        path: String::new(),
        title: "Not found".into(),
        desc: "Page not found.".into(),
        og: "og/home.png".into(),
        nav: "",
        class: "p-404",
        body,
        scripts: &["site.js"],
        head_extra: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use crate::html::{Ctx, Links};
    use crate::model::Data;
    use crate::view::Model;

    #[test]
    fn edition_body_renders_as_paragraphs_on_its_own_page_only() {
        let mut d = Data::load(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/data"))).expect("data loads");
        let last = d.radars[0].editions.len() - 1;
        d.radars[0].editions[last].body = Some("First paragraph with [a link](https://example.org).\n\nSecond\nparagraph.".into());
        let m = Model::new(&d);
        let ctx = Ctx { m: &m, links: Links::Dirs, css: String::new(), base_url: "https://example.org".into(), versions: vec![] };
        let page = super::edition(&ctx, last).body;
        let art = "<div class=\"ed-article prose\"><p>First paragraph with <a href=\"https://example.org\" rel=\"noopener\">a link</a>.</p><p>Second\nparagraph.</p></div>";
        assert!(page.contains(art), "{page}");
        assert!(page.find("ed-note").unwrap() < page.find(art).unwrap());
        assert!(!super::moves(&ctx).body.contains("First paragraph"));
        assert!(crate::feeds::atom(&ctx).contains("Second\nparagraph."));
        assert!(crate::feeds::llms(&ctx).contains("Second paragraph."));
        assert!(crate::feeds::radar_json(&ctx).contains("\"body\":\"First paragraph"));
    }

    #[test]
    fn first_para_is_one_line() {
        assert_eq!(crate::html::first_para("\n  One\n line.\n\nTwo."), "One line.");
        assert_eq!(crate::html::para_lines("One\nline.\n\n\n\nTwo."), vec!["One line.", "Two."]);
    }
}
