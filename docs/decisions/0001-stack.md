# 0001: Stack for the foundation build

**Status:** built as proposed in brief 001. Owner to confirm.
**Date:** 2026-09-29

## Decision

- **Data:** plain TOML in `data/`. One file per entry (facts), one file per radar (opinions, editions, history). A personal radar is a new file, not a code change.
- **Generator:** a purpose-built Rust binary (`src/`, about 3,200 lines including tests). It validates the data, lays out the radar, and writes static HTML with inline SVG, feeds, badges and social cards.
- **Client JS:** two small vanilla files. `site.js` (about 9 KB) adds hover, zoom, replay and the sweep. `build.js` (about 17 KB) runs the radar builder. Everything except the builder works without JS.
- **Hosting:** Cloudflare Pages via Wrangler. The build writes a `_headers` file for caching.

## Why

- **Machine-checkable feedback.** The compiler, 11 tests and the data validator catch agent mistakes before anything is written. Unknown TOML fields, dangling references, undated moves and moves without a reason all fail the build.
- **Lean.** Three dependencies: `serde`, `toml`, `resvg`. No Node toolchain. A full build, including 60 social-card PNGs, takes about 0.4 s.
- **Fast pages.** The home page is about 19 KB gzipped, with CSS inlined and fonts preloaded. Lighthouse (mobile, simulated throttling): performance 98 to 99, accessibility 100, best practices 100, SEO 100.

## Trade-offs

- **`resvg` is the one heavy dependency.** It adds about 45 s to a cold release build. It exists for social cards, which are the unit people share (PNG is required by X, LinkedIn and others). `--no-og` skips it at run time. If compile time starts to hurt, move card rendering behind a Cargo feature.
- **Layout logic is duplicated.** The builder's live preview reimplements the radar layout in JS (`assets/build.js`, mirroring `src/layout.rs`). Keep the two in step. The constants are marked in both files.
- **Hand-written HTML templates.** Pages are `format!` strings, not a template engine. That's fine for agents and cheap to compile. If page count or complexity grows a lot, reconsider `maud` for compile-checked markup.

## Fallback

Brief 001's fallback (Astro or Zola) was not needed. The generator was done well inside the budget.
