# Product Radar

**productradar.org** is an opinionated, regularly updated radar of the tools, technologies and practices that matter to AI-enabled builders right now.

Built and curated by [Konstantin Kaufmann](https://kkauf.com). Live at **[productradar.org](https://productradar.org)**.

> **Snapshot.** This public repository is a read-only snapshot of the site code and radar data as of 2026-11-24. The working repository, with its agent runs and review notes, stays private. This copy is not kept in sync.

It is inspired by the [Thoughtworks Technology Radar](https://www.thoughtworks.com/radar), but has a different audience and a different question.

## Why it exists

AI is merging engineering and product management. A single person with good agents can now take a customer problem all the way to a working business. That used to take a team.

The bottleneck has moved. It is less about the *craft* of building software and more about:

- finding and understanding customer problems,
- executing on them fast,
- turning them into valuable, sustainable businesses.

The tool landscape behind this changes every week, and most coverage is either hype or written for specialist engineers. Product Radar is a curated, first-hand view for people who build *businesses*, not just software.

## Audience

- **Solopreneurs and indie founders** who build and run products largely alone, with AI.
- **Product managers** whose role is expanding into building, because agents now do much of the implementation.

- **Software engineers moving the other way**, towards product, user craft and design, as agents take over more of the coding.

The roles are converging. All three groups care about leverage, speed and outcomes. The technical depth of a tool matters less to them.

## What's on the radar

Anything that materially changes what one AI-enabled person can build and ship. Examples of the range:

| Kind | Examples |
|------|----------|
| AI models | Coding models (e.g. Claude Opus 5.5), voice models (e.g. ElevenLabs' latest) |
| Agentic dev tools | Claude Code, T3 Code |
| Agent-friendly infrastructure | Cloudflare CLI: lets agents manage domains, DNS and hosting directly |
| Hosting & deployment | Ways to host and ship with minimal ops |
| Local / on-device AI | Running and interacting with models on your own machine |
| Practices | Workflows and techniques for building with agents |
| Business operations | Agent orchestration for running a company (e.g. Paperclip), AI-native CRMs, "company brain" tools, personal knowledge bases (e.g. Obsidian) |

**Signal over noise.** The space produces more launches than anyone can track, and a radar that lists everything is useless. An entry earns its place only if it changes what one AI-enabled person can build, ship or run. Novelty alone isn't enough.

### Quadrants (working draft)

| Quadrant | Contains |
|----------|----------|
| **Models & Agents** | Foundation models and the agents built directly on them (coding, voice, image, reasoning) |
| **Tools & Harnesses** | Apps and CLIs you work *through*: agent harnesses (Claude Code, T3 Code), business tools, CRMs, knowledge bases |
| **Infrastructure** | Hosting, deployment, data, and agent-friendly platform interfaces (e.g. Cloudflare CLI), local/on-device AI runtimes |
| **Practices** | Techniques and workflows: how to build, validate and run a business with agents |

Entries are placed in rings (a Thoughtworks-style convention):

- **Adopt**: proven in real use; a default choice.
- **Trial**: worth using on real work where the risk is manageable.
- **Assess**: worth understanding and watching.
- **Hold**: avoid for new work, or superseded.

## Editorial model

- **Curated by one practitioner**, based on what is actually used to build and run real businesses, plus what is moving in the wider market. It is first-hand, opinionated and not neutral.
- **Weekly editions, shipped behind guardrails.** Every Friday a scheduled agent run reads the week's working sessions, drafts the moves and a short article for each, and has independent reviewer agents check them against [`docs/acceptance.md`](docs/acceptance.md). When every check passes, the edition ships without the owner's review. Every change records *why*.
- **Movement is the signal.** What moved, and why, is often more useful than a static snapshot.

## Direction: from one radar to many

Product Radar starts as one curated radar. The longer-term idea is that **anyone can keep their own radar** of the stack they actually use, keep it current, and compare it with others. My "Adopt" isn't necessarily yours, and that disagreement is useful signal in itself.

This is deliberately **not** a Product Hunt competitor. It isn't about launches. It's about shaping the conversation on which tools people actually use, and why.

The closest precedent is [StackShare](https://stackshare.io) (acquired by FOSSA in 2024). Its weakness shows the opportunity: stack profiles are one-time submissions that go stale ("updated 2 years ago"). A radar is maintained, has a history, and records *movement and reasons*, which is exactly what a stale stack profile lacks.

Sequencing: the curated radar comes first. Personal radars come next, starting as files in this repo (no accounts, no database) to test demand before building any social features.

## The project is also the experiment

The stack for this repo is chosen for **AI-authored code, not human-authored code**:

- **Optimize for:** runtime performance, a lean footprint, token efficiency, and strong machine-checkable feedback (compilers, type systems, tests) that lets agents verify their own work.
- **Don't optimize for:** human readability or familiarity. Code is written and maintained mainly by agents. A language or framework that humans find painful but agents handle well is a valid choice. Rust is the reference case: it's verbose, so it isn't token-efficient, but its compiler catches agent mistakes and its servers are dramatically cheaper and faster to run.
- **Eat our own cooking.** Stack decisions here are radar material. If a tool is used to build this site, it should probably be on the radar, with an honest verdict.

## Status

**Version 1 published** at [productradar.org](https://productradar.org). Edition 02 of 2026-10-22 is the first weekly edition: an agent drafted it from the owner's sessions, reviewer agents checked it against `docs/acceptance.md`, and it shipped without his review. The site, the data model and the generator work end to end.

- The curated radar is **Konstantin's radar**. Its name lives in one place: `title` in `data/radars/product-radar.toml`. The site itself stays "Product Radar".
- It holds 46 entries from the owner's own stack. 38 are on the radar (Models 9, Tools 10, Infrastructure 11, Practices 8): Adopt 21, Trial 10, Assess 2, Hold 5. 8 practices are retired. Every verdict has `status = "confirmed"`. 41 entries cite outside sources; 5 practices come from the owner's own work and have none.
- Edition 01 (2026-10-12) is the starting point. Pages, the feed and the edition card show it as ring counts, never as 41 "new" rows. From edition 02 on, each edition lists its moves.
- The one community radar is still a fictional example.

What exists:

- **The radar**, an instrument-style scope with quadrant zoom, hover data tags and a replay of every edition. Strong colour marks only what changed. The rings carry a soft teal tint.
- **Entry pages** with a verdict, a quadrant close-up, a ring-over-time chart, placements on other radars, sources, share links and an embeddable badge.
- **What moved**: a changelog of editions, each with its own permalink and social card.
- **Build your radar**: a no-account builder. It starts from the rings on Konstantin's radar, and the share link carries the whole radar. It also exports a PNG and a TOML file for a pull request.
- **Community radars** (`/r/<id>/`), compared with the curated one.
- **Open data**: `radar.json`, `llms.txt`, an Atom feed, a sitemap and per-entry badges.

### Growth loops built in

The radar grows through what people share, not through launches:

1. **Disagreement → personal radar → shared link.** Every entry page and the home page lead to the builder. It opens pre-filled, so disagreeing takes one click. The shared view shows where that person differs from the curated radar.
2. **Makers embed badges.** Each entry has a live SVG badge with its ring, plus copy-paste Markdown and HTML. Tools on the radar link back.
3. **Every page is a shareable card.** Entries, editions and radars each get a 1200×630 social card showing the blip, its ring and its movement.
4. **Weekly editions are the news hook.** Each edition has a permalink, a card and a feed entry. "What moved this week" is the recurring post.
5. **Agents and search as distribution.** `llms.txt`, `radar.json` and clean per-entry pages make the radar quotable by assistants and findable by search.

Not built yet, on purpose: accounts, comments, voting and a hosted database. Personal radars start as share links and as files in `data/radars/` via pull request, to test demand first.

## Working on it

```
cargo run --release -- check               # validate data/ only (includes the leak check)
cargo run --release -- check --ship        # ship gate: also fails on any draft blip on the curated radar
cargo test                                 # unit tests + validates the real data
cargo run --release -- build               # writes dist/ (about 1 s, including social cards)
cargo run --release -- build --no-og       # skip social-card PNGs
cargo run --release -- build --links files # links that work from file:// or plain file hosts
python3 -m http.server -d dist 8000        # preview
npx wrangler pages deploy dist --project-name product-radar   # deploy (needs CLOUDFLARE_API_TOKEN)
```

- Acceptance criteria for every edition: [`docs/acceptance.md`](docs/acceptance.md)
- Design system and its rules: [`docs/design.md`](docs/design.md)
- Stack decision: [`docs/decisions/0001-stack.md`](docs/decisions/0001-stack.md)
- The weekly agent run (session reader, X-bookmark intake, reviewer agents) lives in the private working repository.
