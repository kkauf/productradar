# Acceptance criteria for an edition

An edition ships without the owner reading it (the owner, 2026-10-22). These criteria are the guardrail. The weekly run checks every edition against them, with machine checks first and independent reviewer agents second. An edition with a blocking finding does not ship (see the pass rule below). The run writes the failure to the owner instead.

Each reviewer gets the built pages and this file. A reviewer never gets the writer's plan or reasoning, so it reads the edition as a visitor does.

## Pass rule

Each reviewer finding has a severity and a scope.

- **Blocking:** every A and B finding. A C or D finding blocks when a visitor would misread a move, could not tell what an entry does for them, or would doubt the radar. Examples: the main point of an entry is unclear on a first read (C1), an entry does not say what it changes (C2), a move has no why (C4), one thing has two names that read as two things (D3).
- **Minor:** every other finding, such as one long sentence or a term a reader can guess from its context.
- **Scope:** a reviewer judges the text that this edition wrote or changed. A finding on older text gets scope `older`. It never blocks. The run lists it in `review.md` in the run folder, and next week's planner treats it as a candidate for a rewrite.
- **Pass:** no reviewer has a blocking finding in scope. The fixer also applies the minor findings it can fix in place.
- **Re-review:** a reviewer gets its earlier findings and checks each one. It adds a new finding only if that finding blocks.

Why (2026-10-22, Edition 02): with "any finding fails", three review rounds found 72, then 56, then 39 findings. Each round found new minor ones, so no edition would ever ship.

## A. Grounded (fact checker)

1. **A1 Owner's own verdict.** Every new ring, move and retirement traces to the owner's own words or to his observed use in the week's records: a session, a decision card, a commit. The private run log names the source for each one, with the date and his exact words. No source: the blip stays `draft`, and the question goes to the owner.
2. **A2 No invented use.** The text claims only the use the records show. "I tried it once" stays "I tried it once".
3. **A3 Outside claims cite a public page.** A claim about a product (what it does, its price, a benchmark, a version) has an `[[evidence]]` item with a public URL and the date it was checked. The checker opens each new URL.
4. **A4 The ring fits its definition.** Adopt: proven in real use and a default choice. Trial: used on real work where the risk is manageable. Assess: tried or watched, too early for a verdict. Hold: avoid for new work.

## B. No internals (leak auditor)

The radar shows how the owner works. It never shows what he works on in private.

1. **B1 No private people.** No names or details of clients, patients, practitioners, family members, candidates or partners' staff. Public figures and the makers of a tool are fine.
2. **B2 No business internals.** No revenue, prices paid, deal terms, customer counts, unreleased plans, partner names or internal project code names. Public products may be named: Kaufmann Health, isaiconsciousyet.com, kkauf.com, this radar.
3. **B3 No security details.** No file paths, host names, secret names, hook or guard internals, ticket ids, private repository names or account names.
4. **B4 No incident details that expose a system.** An outage or failure is told as the lesson ("a second host kept the site up"), never as a map of the system.
5. **B5 Fair to vendors.** A Hold verdict states the owner's own experience and context ("it failed on my Mac", "too costly for my use"). It never states a general judgment of a product without a public source.
6. **B6 Machine check.** `cargo run --release -- check --ship` passes, including the leak check with the private denylist.

## C. Clear for the audience (reader personas)

The audience: solopreneurs, product managers who now build with agents, and engineers who move towards product (README § Audience). Most of them do not write code.

1. **C1 First read.** A product manager understands each text on the first read. Each technical term is explained in plain words, or replaced.
2. **C2 What it does for a business.** Each entry says what the thing changes for one person who builds, ships or runs a product. A tool that only matters to specialist engineers does not go on the radar.
3. **C3 Signal over noise.** At most 5 new entries per edition. Novelty alone is not a reason.
4. **C4 A move says why.** Each move has a one-line note. Each changed blip has a short article in its `rationale`: how the owner uses it, why it sits in this ring and, where it helps, what would move it. Two to four short paragraphs, at most 900 characters.
5. **C5 The edition tells the week.** The edition has a title, a note of at most two sentences, and a `body` of one to three paragraphs about what moved and why.

## D. Style (style checker)

1. **D1 Plain English.** One idea per sentence, at most 25 words. Active voice. First person for the owner's verdicts. Simple tenses. No semicolons.
2. **D2 No hype and no AI tells.** No "it's not X, it's Y" reveals, no lists of three for rhythm, no "game-changer", no sentence that trails into extra comma clauses.
3. **D3 One term per concept.** The same thing has the same name on every page.

## E. Build (machine checks, before any reviewer)

1. **E1** `cargo run --release -- check --ship` has no errors and no draft blips.
2. **E2** `cargo test --release` passes.
3. **E3** `cargo run --release -- build` writes `dist/`, and the edition page, the changed entry pages, `/radar/` and the home page render.
4. **E4** After the deploy, the live edition page at `https://productradar.org/moves/<date>/` returns 200 and shows the edition title.
