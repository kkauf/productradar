//! HTML primitives: escaping, a tiny inline-markdown subset, relative links and the page shell.

use crate::view::Model;
use std::fmt::Write;

pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            _ => o.push(c),
        }
    }
    o
}

/// Escapes text, then renders `[label](https://…)` links and `code` spans. Nothing else.
pub fn inline(s: &str) -> String {
    let mut o = String::new();
    let mut rest = s;
    while !rest.is_empty() {
        if let Some(stripped) = rest.strip_prefix('[')
            && let Some(close) = stripped.find("](")
        {
            let label = &stripped[..close];
            let after = &stripped[close + 2..];
            if let Some(end) = after.find(')') {
                let url = &after[..end];
                if url.starts_with("https://") && !label.is_empty() {
                    let _ = write!(o, "<a href=\"{}\" rel=\"noopener\">{}</a>", esc(url), esc(label));
                    rest = &after[end + 1..];
                    continue;
                }
            }
        }
        if let Some(stripped) = rest.strip_prefix('`')
            && let Some(end) = stripped.find('`')
        {
            let _ = write!(o, "<code>{}</code>", esc(&stripped[..end]));
            rest = &stripped[end + 1..];
            continue;
        }
        let c = rest.chars().next().unwrap();
        o.push_str(&esc(&c.to_string()));
        rest = &rest[c.len_utf8()..];
    }
    o
}

/// Paragraphs split on blank lines.
/// The first paragraph of a text whose paragraphs are separated by blank lines, on one line.
/// For places that hold one line: hover tags, social cards, meta descriptions, Focus box lines.
pub fn first_para(s: &str) -> String {
    s.split("\n\n").map(str::trim).find(|p| !p.is_empty()).unwrap_or("").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every paragraph of a text, each on one line, for plain-text formats.
pub fn para_lines(s: &str) -> Vec<String> {
    s.split("\n\n").map(|p| p.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|p| !p.is_empty()).collect()
}

pub fn paras(s: &str) -> String {
    s.split("\n\n").map(str::trim).filter(|p| !p.is_empty()).map(|p| format!("<p>{}</p>", inline(p))).collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Links {
    /// `e/claude-code/`: pretty URLs for real hosting.
    Dirs,
    /// `e/claude-code/index.html`: works from file:// and plain file hosts.
    Files,
}

pub struct Ctx<'a> {
    pub m: &'a Model<'a>,
    pub links: Links,
    pub css: String,
    pub base_url: String,
    /// Content hashes appended to script URLs, keyed by file name.
    pub versions: Vec<(&'static str, String)>,
}

pub struct Page {
    /// Site path of the page, "" for home, "e/x/" for others.
    pub path: String,
    pub title: String,
    pub desc: String,
    /// Site path of the social card PNG.
    pub og: String,
    pub nav: &'static str,
    pub class: &'static str,
    pub body: String,
    pub scripts: &'static [&'static str],
    pub head_extra: String,
}

impl Ctx<'_> {
    /// Relative href from page `from` to site path `to`.
    pub fn href(&self, from: &str, to: &str) -> String {
        let depth = from.matches('/').count();
        let mut s = "../".repeat(depth);
        s.push_str(to);
        if self.links == Links::Files && (to.is_empty() || to.ends_with('/')) {
            s.push_str("index.html");
        }
        if s.is_empty() { "./".into() } else { s }
    }

    pub fn abs(&self, to: &str) -> String {
        format!("{}/{}", self.base_url, to)
    }

    pub fn render(&self, p: &Page) -> String {
        let m = self.m;
        let main = m.main();
        let latest = main.latest();
        let date = main.dates[latest];
        let abs_root = p.class == "p-404";
        let root = if abs_root { "/".to_string() } else { self.href(&p.path, "") };
        let root = root.trim_end_matches("index.html");
        let rootdir = if root == "./" { "" } else { root };
        let link = |to: &str| if abs_root { format!("/{to}") } else { self.href(&p.path, to) };
        let css = self.css.replace("__ROOT__", rootdir);
        let full_title = if p.path.is_empty() { format!("{} · {}", m.site.title, m.site.tagline) } else { format!("{} · {}", p.title, m.site.title) };
        let mut h = String::with_capacity(64 * 1024);
        let _ = write!(
            h,
            r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">
<title>{t}</title>
<meta name="description" content="{d}">
<link rel="canonical" href="{canon}">
<meta property="og:type" content="website">
<meta property="og:site_name" content="{site}">
<meta property="og:title" content="{ot}">
<meta property="og:description" content="{d}">
<meta property="og:url" content="{canon}">
<meta property="og:image" content="{og}">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">
<meta name="twitter:card" content="summary_large_image">
<meta name="theme-color" content="#eef1f2" media="(prefers-color-scheme: light)">
<meta name="theme-color" content="#0d1419" media="(prefers-color-scheme: dark)">
<link rel="icon" href="{rootdir}favicon.svg" type="image/svg+xml">
<link rel="apple-touch-icon" href="{rootdir}apple-touch-icon.png">
<link rel="alternate" type="application/atom+xml" title="{feed}" href="{rootdir}feed.xml">
<link rel="preload" href="{rootdir}fonts/archivo.woff2" as="font" type="font/woff2" crossorigin>
<script>try{{var t=localStorage.getItem("pr-theme");if(t)document.documentElement.setAttribute("data-theme",t)}}catch(e){{}}</script>
<style>{css}</style>
{extra}</head>
<body class="{class}">
<a class="skip" href="#main">Skip to content</a>
<header class="top"><div class="wrap top-in">
<a class="mark" href="{root}" aria-label="{site} home">{glyph}<span>{site}</span></a>
<nav class="nav" aria-label="Main">"##,
            t = esc(&full_title),
            d = esc(&p.desc),
            canon = esc(&self.abs(&p.path)),
            site = esc(&m.site.title),
            feed = esc(&crate::feeds::feed_title(m)),
            ot = esc(&p.title),
            og = esc(&self.abs(&p.og)),
            rootdir = rootdir,
            css = css,
            extra = p.head_extra,
            class = p.class,
            root = root,
            glyph = MARK_SVG,
        );
        for (key, path, label) in [("radar", "radar/", "Radar"), ("moves", "moves/", "What moved"), ("build", "build/", "Build yours"), ("about", "about/", "About")] {
            let cur = if key == p.nav { " aria-current=\"page\"" } else { "" };
            let _ = write!(h, "<a href=\"{}\"{cur}>{label}</a>", link(path));
        }
        let _ = write!(
            h,
            r##"</nav>
<a class="stamp" href="{mv}" title="Latest edition">{stamp}</a>
<button class="theme" type="button" aria-label="Switch colour theme"><svg viewBox="0 0 20 20" aria-hidden="true"><circle cx="10" cy="10" r="7.5"/><path d="M10 2.5a7.5 7.5 0 0 1 0 15z"/></svg></button>
</div></header>
"##,
            mv = link(&format!("moves/{}/", date)),
            stamp = esc(&edition_name(latest, date)),
        );
        let _ = write!(h, "<main id=\"main\">\n{}\n</main>\n", p.body);
        let _ = write!(
            h,
            r##"<footer class="foot"><div class="wrap foot-in">
<div class="foot-about"><p class="foot-mark">{glyph}<span>{site}</span></p><p>{desc}</p></div>
<div><h2 class="label">Follow</h2><ul><li><a href="{moves}">What moved</a></li><li><a href="{feed}">Atom feed</a></li><li><a href="{build}">Build your radar</a></li></ul></div>
<div><h2 class="label">Open data</h2><ul><li><a href="{json}">radar.json</a></li><li><a href="{llms}">llms.txt</a></li><li><a href="{about}#method">Method</a></li></ul></div>
<div><h2 class="label">Built with</h2><ul><li><a href="{e1}">Claude Code</a></li><li><a href="{e2}">Rust</a></li><li><a href="{e3}">Cloudflare Pages</a></li></ul></div>
</div></footer>
"##,
            glyph = MARK_SVG,
            site = esc(&m.site.title),
            desc = esc(&m.site.description),
            moves = link("moves/"),
            feed = link("feed.xml"),
            build = link("build/"),
            json = link("radar.json"),
            llms = link("llms.txt"),
            about = link("about/"),
            e1 = link("e/claude-code/"),
            e2 = link("e/compiler-feedback/"),
            e3 = link("e/cloudflare-pages/"),
        );
        for s in p.scripts {
            let v = self.versions.iter().find(|(n, _)| n == s).map(|(_, v)| format!("?v={v}")).unwrap_or_default();
            let _ = writeln!(h, "<script src=\"{}{v}\" defer></script>", link(s));
        }
        h.push_str("</body>\n</html>\n");
        h
    }
}

/// The one name of an edition, on every page and card: "Edition 01 · 12 Oct 2026".
pub fn edition_name(e: usize, d: crate::date::Date) -> String {
    format!("Edition {:02} · {}", e + 1, d.human())
}

/// The wordmark glyph: a quadrant scope with one blip.
pub const MARK_SVG: &str = r#"<svg class="glyph" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="10.5"/><circle cx="12" cy="12" r="6"/><path d="M12 1.5V22.5M1.5 12H22.5"/><circle class="glyph-blip" cx="16.2" cy="7.8" r="2.4"/></svg>"#;

/// Collapses CSS whitespace and comments. Good enough for our own hand-written stylesheet.
pub fn minify_css(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    let mut last_space = false;
    while let Some(c) = chars.next() {
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut prev = ' ';
            for c2 in chars.by_ref() {
                if prev == '*' && c2 == '/' {
                    break;
                }
                prev = c2;
            }
            continue;
        }
        if c.is_whitespace() {
            last_space = true;
            continue;
        }
        if last_space {
            let prev = out.chars().last().unwrap_or('{');
            if !"{};:,>".contains(prev) && !"{};,>".contains(c) {
                out.push(' ');
            }
            last_space = false;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_links_and_escaping() {
        assert_eq!(inline("a <b> [x](https://e.org) `c`"), "a &lt;b&gt; <a href=\"https://e.org\" rel=\"noopener\">x</a> <code>c</code>");
        assert_eq!(inline("[x](javascript:alert)"), "[x](javascript:alert)");
    }

    #[test]
    fn css_minify_keeps_descendant_spaces() {
        assert_eq!(minify_css("a  b { color: red; }\n/* c */ .x > .y{margin:calc(1px - 2px)}"), "a b{color:red;}.x>.y{margin:calc(1px - 2px)}");
    }
}
