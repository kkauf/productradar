//! Machine-readable outputs: Atom feed, sitemap, robots, radar.json and llms.txt.

use crate::html::{Ctx, esc, para_lines, paras};
use crate::pages::json_str;
use crate::view::{Model, RadarView, State};
use std::fmt::Write;

/// Feed title, shared by the feed itself and the `<link rel="alternate">` in every page head.
pub fn feed_title(m: &Model) -> String {
    format!("{}: what moved on {}", m.site.title, m.curated())
}

/// Edition 01 is the starting point: entries per ring, e.g. "19 in Adopt, 14 in Trial, 2 in Assess, 6 in Hold".
pub fn ring_summary(m: &Model, rv: &RadarView, e: usize) -> String {
    let mut n = [0usize; 4];
    for (_, b) in rv.on_radar(e) {
        if let Some(k) = b.rings[e] {
            n[k] += 1;
        }
    }
    n.iter().enumerate().map(|(k, c)| format!("{c} in {}", m.ring_name(k))).collect::<Vec<_>>().join(", ")
}

pub fn atom(ctx: &Ctx) -> String {
    let m = ctx.m;
    let rv = m.main();
    let updated = rv.dates[rv.latest()];
    let mut o = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<feed xmlns=\"http://www.w3.org/2005/Atom\"><title>{t}</title><subtitle>{s}</subtitle><link href=\"{home}\"/><link rel=\"self\" href=\"{self_}\"/><id>{home}</id><updated>{updated}T09:00:00Z</updated><author><name>{a}</name></author>\n",
        t = esc(&feed_title(m)),
        s = esc(&m.site.tagline),
        a = esc(&rv.r.owner),
        home = esc(&ctx.abs("")),
        self_ = esc(&ctx.abs("feed.xml")),
    );
    for e in (0..rv.dates.len()).rev().take(20) {
        let d = rv.dates[e];
        let ed = rv.edition(e);
        let mut html = format!("<p>{}</p>", esc(&ed.note));
        if let Some(t) = &ed.body {
            html.push_str(&paras(t));
        }
        if e == 0 {
            // Edition 01 is the starting point: ring counts, never a list of 40 "new" rows.
            let _ = write!(
                html,
                "<p>The starting point: {} entries. {}. Nothing has moved yet.</p><p><a href=\"{}\">Open {}</a></p>",
                rv.on_radar(0).count(),
                ring_summary(m, rv, 0),
                esc(&ctx.abs("radar/")),
                esc(m.curated())
            );
        } else {
            html.push_str("<ul>");
            for mv in rv.moves_at(e) {
                let b = &rv.blips[mv.blip];
                let what = match (mv.from, mv.to) {
                    (None, Some(t)) => format!("new in {}", m.ring_name(t)),
                    (Some(f), None) => format!("retired from {}", m.ring_name(f)),
                    (Some(f), Some(t)) => format!("{} → {}", m.ring_name(f), m.ring_name(t)),
                    _ => String::new(),
                };
                let _ = write!(html, "<li><a href=\"{}\">{}</a>: {}. {}</li>", esc(&ctx.abs(&format!("e/{}/", b.e.id))), esc(&b.e.name), esc(&what), esc(mv.note));
            }
            html.push_str("</ul>");
        }
        let url = ctx.abs(&format!("moves/{d}/"));
        let _ = writeln!(
            o,
            "<entry><title>Edition {:02}: {}</title><link href=\"{u}\"/><id>{u}</id><updated>{d}T09:00:00Z</updated><content type=\"html\">{}</content></entry>",
            e + 1,
            esc(&ed.title),
            esc(&html),
            u = esc(&url),
        );
    }
    o.push_str("</feed>\n");
    o
}

pub fn sitemap(ctx: &Ctx, paths: &[String]) -> String {
    let rv = ctx.m.main();
    let last = rv.dates[rv.latest()];
    let mut o = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    for p in paths {
        let _ = writeln!(o, "<url><loc>{}</loc><lastmod>{last}</lastmod></url>", esc(&ctx.abs(p)));
    }
    o.push_str("</urlset>\n");
    o
}

pub fn robots(ctx: &Ctx) -> String {
    format!("User-agent: *\nAllow: /\n\nSitemap: {}\n", ctx.abs("sitemap.xml"))
}

/// The open-data export: everything needed to rebuild or remix the curated radar.
pub fn radar_json(ctx: &Ctx) -> String {
    let m = ctx.m;
    let rv = m.main();
    let e = rv.latest();
    let mut o = format!(
        "{{\n\"radar\":{},\n\"owner\":{},\n\"url\":{},\n\"edition\":{{\"number\":{},\"date\":\"{}\",\"title\":{},\"note\":{},\"body\":{}}},\n",
        json_str(&rv.r.title),
        json_str(&rv.r.owner),
        json_str(&ctx.abs("radar/")),
        e + 1,
        rv.dates[e],
        json_str(&rv.edition(e).title),
        json_str(&rv.edition(e).note),
        rv.edition(e).body.as_deref().map(json_str).unwrap_or_else(|| "null".into())
    );
    let _ =
        writeln!(o, "\"quadrants\":[{}],", m.site.quadrants.iter().map(|q| format!("{{\"id\":{},\"name\":{}}}", json_str(&q.id), json_str(&q.name))).collect::<Vec<_>>().join(","));
    let _ = write!(
        o,
        "\"rings\":[{}],\n\"entries\":[\n",
        m.site.rings.iter().map(|r| format!("{{\"id\":{},\"name\":{}}}", json_str(&r.id), json_str(&r.name))).collect::<Vec<_>>().join(",")
    );
    let mut first = true;
    for b in &rv.blips {
        let ring = b.ring_now().map(|k| json_str(&m.site.rings[k].id)).unwrap_or_else(|| "\"retired\"".into());
        let hist: Vec<String> = b.b.history.iter().map(|h| format!("{{\"date\":\"{}\",\"ring\":{},\"note\":{}}}", h.date, json_str(&h.ring), json_str(&h.note))).collect();
        // Edition 01 is the starting point: nothing on it counts as a change.
        let st = if e == 0 { State::Steady } else { b.state_at(e) };
        let _ = write!(
            o,
            "{}{{\"id\":{},\"name\":{},\"quadrant\":{},\"ring\":{ring},\"change\":{},\"status\":{},\"summary\":{},\"rationale\":{},\"url\":{},\"page\":{},\"history\":[{}]}}",
            if first { "" } else { ",\n" },
            json_str(&b.e.id),
            json_str(&b.e.name),
            json_str(&b.e.quadrant),
            json_str(match st {
                State::New => "new",
                State::In => "in",
                State::Out => "out",
                State::Retired => "retired",
                _ => "none",
            }),
            json_str(if b.b.status == crate::model::Status::Draft { "draft" } else { "confirmed" }),
            json_str(&b.e.summary),
            json_str(&b.b.rationale),
            b.e.url.as_deref().map(json_str).unwrap_or_else(|| "null".into()),
            json_str(&ctx.abs(&format!("e/{}/", b.e.id))),
            hist.join(",")
        );
        first = false;
    }
    o.push_str("\n]\n}\n");
    o
}

pub fn llms(ctx: &Ctx) -> String {
    let m = ctx.m;
    let rv = m.main();
    let e = rv.latest();
    let mut o = format!(
        "# {}\n\n> {}\n\nThis file lists {}, edition {:02} ({}): {} entries. {}.\n\n- The radar: {}\n- Full data as JSON: {}\n",
        m.site.title,
        m.site.description,
        m.curated(),
        e + 1,
        rv.dates[e].human(),
        rv.on_radar(e).count(),
        ring_summary(m, rv, e),
        ctx.abs("radar/"),
        ctx.abs("radar.json")
    );
    for (k, r) in m.site.rings.iter().enumerate() {
        let _ = write!(o, "\n## {}\n\n{}\n\n", r.name, r.description);
        for (q, quad) in m.site.quadrants.iter().enumerate() {
            for (_, b) in rv.on_radar(e).filter(|(_, b)| b.q == q && b.rings[e] == Some(k)) {
                // A multi-paragraph rationale: the first paragraph on the item line, the rest as indented paragraphs of the item.
                let ps = para_lines(&b.b.rationale);
                let _ = writeln!(o, "- [{}]({}) ({}): {} {}", b.e.name, ctx.abs(&format!("e/{}/", b.e.id)), quad.name, b.e.summary, ps.first().map(String::as_str).unwrap_or(""));
                for p in ps.iter().skip(1) {
                    let _ = writeln!(o, "\n  {p}");
                }
            }
        }
    }
    let ed = rv.edition(e);
    let _ = write!(o, "\n## Changes in edition {:02}\n\n", e + 1);
    for p in para_lines(&ed.note).iter().chain(para_lines(ed.body.as_deref().unwrap_or("")).iter()) {
        let _ = write!(o, "{p}\n\n");
    }
    if e == 0 {
        o.push_str("None. Edition 01 is the starting point.\n");
    } else {
        for mv in rv.moves_at(e) {
            let b = &rv.blips[mv.blip];
            let _ = writeln!(o, "- {}: {}. {}", b.e.name, mv.state.label(), mv.note);
        }
    }
    o
}
