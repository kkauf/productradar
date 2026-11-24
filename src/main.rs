//! productradar.org generator.
//!
//!   radar check                 validate data/ and exit non-zero on errors
//!   radar check --ship          the same, and a draft blip on a curated radar is an error:
//!                               the gate an automated edition passes before it ships
//!   radar build [options]       validate, then write the static site to dist/
//!   radar focus-page [--title T] print the Focus editor review page to stdout
//!
//! Private terms for the leak check: the file at env RADAR_DENYLIST, else
//! ~/.claude/product-radar-data/denylist.txt (never committed).
//!
//! Options: --data DIR, --out DIR, --links dirs|files, --base-url URL, --no-og, --title T

// Index loops read better here than enumerate chains (clippy 1.100 flags them).
#![allow(clippy::needless_range_loop)]

mod date;
mod feeds;
mod focus;
mod html;
mod layout;
mod model;
mod og;
mod pages;
mod svg;
mod validate;
mod view;

use html::{Ctx, Links, Page};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

const CSS: &str = include_str!("../assets/site.css");
const SITE_JS: &str = include_str!("../assets/site.js");
const BUILD_JS: &str = include_str!("../assets/build.js");
const FONTS: [(&str, &[u8]); 4] = [
    ("archivo.woff2", include_bytes!("../assets/fonts/archivo.woff2")),
    ("archivo-arrows.woff2", include_bytes!("../assets/fonts/archivo-arrows.woff2")),
    ("martian-mono.woff2", include_bytes!("../assets/fonts/martian-mono.woff2")),
    ("martian-mono-arrows.woff2", include_bytes!("../assets/fonts/martian-mono-arrows.woff2")),
];
/// Marks a directory as ours, so `build` only ever clears what it wrote.
const MARKER: &str = ".radar-dist";

struct Opts {
    data: PathBuf,
    out: PathBuf,
    links: Links,
    base_url: Option<String>,
    og: bool,
    title: String,
    /// `check --ship`: a draft blip on a curated radar is an error, not a warning.
    ship: bool,
}

fn parse_args(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts { data: "data".into(), out: "dist".into(), links: Links::Dirs, base_url: None, og: true, title: "Product Radar".into(), ship: false };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut val = || it.next().cloned().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--data" => o.data = val()?.into(),
            "--out" => o.out = val()?.into(),
            "--base-url" => o.base_url = Some(val()?.trim_end_matches('/').to_string()),
            "--links" => {
                o.links = match val()?.as_str() {
                    "dirs" => Links::Dirs,
                    "files" => Links::Files,
                    v => return Err(format!("--links must be dirs or files, not {v}")),
                }
            }
            "--no-og" => o.og = false,
            "--ship" => o.ship = true,
            "--title" => o.title = val()?,
            _ => return Err(format!("unknown option {a}")),
        }
    }
    Ok(o)
}

fn load(opts: &Opts) -> Result<model::Data, ()> {
    let data = model::Data::load(&opts.data).map_err(|errs| {
        for e in errs {
            eprintln!("error: {e}");
        }
    })?;
    let mut rep = validate::validate_with(&data, &validate::load_denylist());
    if opts.ship {
        rep.errors.extend(validate::ship_errors(&data));
    }
    for w in &rep.warnings {
        eprintln!("warning: {w}");
    }
    for e in &rep.errors {
        eprintln!("error: {e}");
    }
    if rep.errors.is_empty() {
        Ok(data)
    } else {
        eprintln!("{} error(s); nothing written", rep.errors.len());
        Err(())
    }
}

/// Short content hash for cache-busting query strings.
fn fnv(s: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{:08x}", h as u32)
}

struct Out {
    root: PathBuf,
    files: usize,
    bytes: usize,
}

impl Out {
    fn write(&mut self, rel: &str, data: &[u8]) -> Result<(), String> {
        let p = self.root.join(rel);
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&p, data).map_err(|e| format!("{}: {e}", p.display()))?;
        self.files += 1;
        self.bytes += data.len();
        Ok(())
    }
    fn page(&mut self, ctx: &Ctx, p: &Page) -> Result<(), String> {
        let html = ctx.render(p);
        let rel = if p.class == "p-404" { "404.html".to_string() } else { format!("{}index.html", p.path) };
        self.write(&rel, html.as_bytes())
    }
}

fn prepare_out(dir: &Path) -> Result<(), String> {
    if dir.exists() {
        let ours = dir.join(MARKER).exists();
        let empty = std::fs::read_dir(dir).map(|mut d| d.next().is_none()).unwrap_or(false);
        if !ours && !empty {
            return Err(format!("{} exists and was not written by this tool; refusing to clear it", dir.display()));
        }
        std::fs::remove_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    std::fs::write(dir.join(MARKER), "").map_err(|e| e.to_string())
}

fn build(opts: &Opts) -> Result<(), String> {
    let t0 = Instant::now();
    let data = load(opts).map_err(|_| String::new())?;
    let m = view::Model::new(&data);
    let js_v = fnv(SITE_JS.as_bytes());
    let build_v = fnv(BUILD_JS.as_bytes());
    let css = html::minify_css(CSS);
    let ctx = Ctx {
        m: &m,
        links: opts.links,
        css,
        base_url: opts.base_url.clone().unwrap_or_else(|| data.site.base_url.clone()),
        versions: vec![("site.js", js_v), ("build.js", build_v)],
    };
    prepare_out(&opts.out)?;
    let mut out = Out { root: opts.out.clone(), files: 0, bytes: 0 };

    let main = m.main();
    let mut pages: Vec<Page> = vec![pages::home(&ctx), pages::radar(&ctx), pages::moves(&ctx), pages::about(&ctx), pages::build(&ctx)];
    for e in 0..main.dates.len() {
        pages.push(pages::edition(&ctx, e));
    }
    for en in &data.entries {
        pages.push(pages::entry(&ctx, &en.id));
    }
    for ri in 1..m.radars.len() {
        pages.push(pages::community(&ctx, ri));
    }
    // Example radars only show how a personal radar compares: search engines skip them.
    let example = |path: &str| m.radars.iter().skip(1).any(|r| r.r.example && path == format!("r/{}/", r.r.id));
    let sitemap: Vec<String> = pages.iter().filter(|p| !example(&p.path)).map(|p| p.path.clone()).collect();
    for p in &pages {
        out.page(&ctx, p)?;
    }
    out.page(&ctx, &pages::not_found(&ctx))?;

    out.write("site.js", SITE_JS.as_bytes())?;
    out.write("build.js", BUILD_JS.as_bytes())?;
    for (name, bytes) in FONTS {
        out.write(&format!("fonts/{name}"), bytes)?;
    }
    out.write("favicon.svg", og::FAVICON.as_bytes())?;
    out.write("feed.xml", feeds::atom(&ctx).as_bytes())?;
    out.write("sitemap.xml", feeds::sitemap(&ctx, &sitemap).as_bytes())?;
    out.write("robots.txt", feeds::robots(&ctx).as_bytes())?;
    out.write("radar.json", feeds::radar_json(&ctx).as_bytes())?;
    out.write("llms.txt", feeds::llms(&ctx).as_bytes())?;
    out.write(
        "_headers",
        b"/fonts/*\n  Cache-Control: public, max-age=31536000, immutable\n/og/*\n  Cache-Control: public, max-age=86400\n/badges/*\n  Cache-Control: public, max-age=3600\n  Access-Control-Allow-Origin: *\n/radar.json\n  Access-Control-Allow-Origin: *\n/*.js\n  Cache-Control: public, max-age=31536000, immutable\n",
    )?;
    for en in &data.entries {
        out.write(&format!("badges/{}.svg", en.id), og::badge(&ctx, &en.id).as_bytes())?;
    }

    if opts.og {
        let r = og::Renderer::new();
        let mut jobs: Vec<(String, String)> = vec![
            ("og/home.png".into(), og::home_card(&ctx)),
            ("og/radar.png".into(), og::radar_card(&ctx, m.curated())),
            (
                "og/moves.png".into(),
                og::generic_card(&ctx, "Every edition", "What moved", &format!("Every change to {}, edition by edition, with the reason for each move.", m.curated())),
            ),
            ("og/about.png".into(), og::generic_card(&ctx, "About", "About Product Radar", "Why the radar exists and how Konstantin Kaufmann decides what goes on it.")),
            (
                "og/build.png".into(),
                og::generic_card(&ctx, "Your radar", "Build your radar", &format!("Place the tools you use in your own rings and see where you disagree with {}.", m.curated())),
            ),
            ("apple-touch-icon.png".into(), og::touch_icon()),
        ];
        for e in 0..main.dates.len() {
            jobs.push((format!("og/moves/{}.png", main.dates[e]), og::edition_card(&ctx, e)));
        }
        for en in &data.entries {
            jobs.push((format!("og/e/{}.png", en.id), og::entry_card(&ctx, &en.id)));
        }
        for ri in 1..m.radars.len() {
            jobs.push((format!("og/r/{}.png", m.radars[ri].r.id), og::community_card(&ctx, ri)));
        }
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(8);
        let chunk = jobs.len().div_ceil(threads);
        let results: Vec<Result<(String, Vec<u8>), String>> = std::thread::scope(|s| {
            let hs: Vec<_> =
                jobs.chunks(chunk).map(|c| s.spawn(|| c.iter().map(|(p, svg)| r.png(svg).map(|b| (p.clone(), b)).map_err(|e| format!("{p}: {e}"))).collect::<Vec<_>>())).collect();
            hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
        });
        for res in results {
            let (p, b) = res?;
            out.write(&p, &b)?;
        }
    }
    eprintln!("built {} files ({} KB) into {} in {:.2}s", out.files, out.bytes / 1024, opts.out.display(), t0.elapsed().as_secs_f64());
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cmd, rest) = match args.split_first() {
        Some((c, r)) => (c.as_str(), r),
        None => ("build", &[][..]),
    };
    let opts = match parse_args(rest) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let res = match cmd {
        "check" => load(&opts).map(|d| eprintln!("ok: {} entries, {} radars", d.entries.len(), d.radars.len())).map_err(|_| String::new()),
        "build" => build(&opts),
        "focus-page" => load(&opts).map(|d| print!("{}", focus::page(&view::Model::new(&d), &opts.title))).map_err(|_| String::new()),
        "fonts" => {
            og::Renderer::new().families().iter().for_each(|f| println!("{f}"));
            Ok(())
        }
        _ => {
            eprintln!("usage: radar [check [--ship]|build|focus-page] [--data DIR] [--out DIR] [--links dirs|files] [--base-url URL] [--no-og] [--title T]");
            return ExitCode::from(2);
        }
    };
    match res {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("error: {e}");
            }
            ExitCode::FAILURE
        }
    }
}
