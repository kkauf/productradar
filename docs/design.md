# Design system

The site is an instrument, not a brochure. The reference is a radar scope and an air-traffic display: precise, calm, and loud only where something changed.

## Concept: movement is the signal

- **Position encodes the verdict.** The quadrant (angle) says what kind of thing an entry is. The ring (radius) says what to do about it.
- **Strong colour encodes change.** Unchanged blips are plain ink. Only new, moved-in and moved-out blips wear a strong colour, so the eye goes to what changed this week. On community radars and in the builder, the same colours encode disagreement with the curated radar: rated higher, rated lower, or only on this radar.
- **Soft colour helps people read.** Each ring has a soft teal tint, strongest at Adopt and faintest at Hold. Panels sit on soft surface layers. See "Colour" below.
- **Trails show history.** A blip that moved in the last few editions draws a line back to the ring it came from, like target history on a radar scope. Older trails fade.
- **Shape backs up colour.** Every state has a shape as well: ◆ new, ▲ in, ▼ out, × retired, ● steady. Colour never carries meaning alone.
- **Ring strength is a signal meter.** Ring chips show four bars: Adopt has four lit, Hold has one and a hatch. This works in monochrome and in both themes.

## Pages and layout

Each page has one job, and nothing is said on two pages.

| Page | Job | Layout |
|---|---|---|
| `/` | Says what Product Radar is, for whom, and which question it answers. Then: how to read a radar, the latest edition (counts and real moves only), build your own, community radars, open data. | Intro (copy beside a picture of the scope that links to `/radar/`), then sections. |
| `/radar/` | Konstantin's radar, the instrument. | Control rail (title, quadrant selector, ring counts, key, replay) beside the scope. The full lists sit below, in four columns. |
| `/e/<id>/` | One entry: verdict, position, movement, other radars, sources, share. | Head (name, ring, quadrant close-up), then sections. On phones the verdict comes before the close-up. The ring chart appears once an entry sits in two editions. Example radars never count as "other radars". |
| `/moves/` | "What moved": every edition, newest first. | The edition stamp sits in the heading rail, the edition beside it. |
| `/moves/<date>/` | One edition. | Edition 01 lists every entry it placed, ring by ring, each name beside its note. Later editions list their moves. |
| `/build/` | The no-account builder. | Entry list with a ring control per row, beside a sticky card: name, share buttons, scope, a row of count cells (same ring, higher, lower, only yours, left off), then the biggest disagreements. The share buttons are in view on the first screen at 1440×900. The card never grows past the screen. On narrow screens the list comes first and a bar at the bottom of the screen counts the changes and shares the link. |
| `/r/<id>/` | A community radar. | The comparison beside the scope, then "Side by side": the entries that differ as a table across the full width, then the entries in the same ring as one short list per ring. |
| `/build/#…` | Someone's shared radar. | The same layout as `/r/<id>/`. "Edit a copy" starts the builder from it. |
| `/about/` | Why it exists, how an entry earns its place, how movement is shown, how the site is built. | Sections. |

- **Sections** (`.sect`): a heading rail on the left (`--rail`, up to 17rem) and the content across the rest of the 1280px column. Home, About and entry pages use them, and the edition stamp on `/moves/` uses the same rail. Content uses multi-column grids (`.cols`, `.dl3`), so no page leaves half the width empty at 1440.
- **Prose keeps its measure.** Body paragraphs stop at `--measure` (about 68 characters), ledes at `--measure-l` (about 64 characters at the larger size). Grids and lists use the full width.
- **The scope fits the screen.** On the radar page the scope's width is capped at the viewport height minus the header, so the whole radar is in view at once.
- **The selector never sits inside the scope's frame.** It lives in the rail (wide screens) or above the scope (narrow screens). Its 2×2 buttons sit where their quadrants sit on the scope.
- **Zoom frames one quadrant.** The zoom shows that quadrant's box plus a 14-unit strip across each axis, and hides the other quadrants' blips. The scope clips to its frame with `clip-path: inset(-8px)`: wide enough for a label that touches the edge, far short of the selector.
- **One name per edition.** Every page, card and the header say "Edition 01 · 12 Oct 2026" (`html::edition_name`). The changelog is "What moved" everywhere.
- **Edition 01 is the baseline.** On the first edition nothing has changed yet, so no blip, list item, summary, entry page or social card wears a change colour or a "new" mark, and the hover tag says nothing about change.

## Scope labels

Every blip on Konstantin's radar has a label in the overview, and so does every blip on a shared radar wider than 640px and in the builder's image export. The test `real_scopes_have_no_label_collisions` (`src/svg.rs`) checks every rule below on the real data, for the overview and each zoomed quadrant.

- **One name.** The scope shows the entry's name in its own case, on two lines when it is long. Where the name does not fit two lines of 18 characters, the scope shows its `label`: the name's first words (the validator checks this). The hover tag and every page show the full name.
- **No numbers on the rim.** The ticks carry no bearing numbers: on this radar a bearing means nothing.
- **Ring sizes.** Adopt is the widest ring: radius 185 of 360. It holds the most entries, and their labels need room. Trial ends at 262, Assess at 316 and Hold at 360.
- **A label stays in its quadrant.** It keeps one font size from the vertical axis, so labels on either side of the axis never run together.
- **A label reads as its own blip's.** Every other blip is at least one font size further from the label's ends, edge centres and corners than the label's own blip. No other blip touches the label or sits in the strip between the label and its blip. No other blip sits level with the label within 3 font sizes past either end (`layout::level_at_end`): "Guard hooks ●" reads as one name and one dot. This holds for labels with a leader too. Without a leader, the label's nearest blip is its own.
- **Preferred spots:** level with the blip (outward, then inward), then above or below it, then a little higher or lower, then at a corner.
- **Stacking.** Labels that sit level with their blips may stack like the rows of a list, 0.6 font sizes apart. Any other label keeps a full font size of space above and below. The lines of one wrapped name touch, so two names never read as one.
- **Leaders are the fallback.** A leader is a hairline in `--muted` from the edge of the blip to the label, at most 4.8 font sizes long. It keeps 3 units clear of every other blip's clearance circle, and it never crosses a label or another leader.
- **Blips move to make room.** Blips spread evenly in their ring cell. Where a label finds no spot, the layout tries other row patterns, then moves single blips to free points of their cell. A blip's place inside its cell carries no meaning. The search keeps labels off the trails of moved blips, as the scope does.
- **When a label does not fit,** the test fails and names the blips without a label. Shorten their `label` first.
- **Shared radars and the image export** (`assets/build.js`) reuse the curated labels (`svg::overview_labels`, emitted as `cat[].L`) for every blip that keeps its curated ring. A blip that moved takes a free point of its cell clear of those labels and of the strips past their ends, then a label by the same rules. A label that still finds no spot goes to the nearest free spot in its quadrant that keeps every rule above, joined by a leader of at most 8 font sizes (14 for a disagreement). Disagreements look first. When a blip is still unnamed, `build.js` tries other orders (nearest the top or bottom first, left to right, right to left, fewest spots first, then 24 shuffled orders with a fixed seed), then moves one or two placed labels to their next spots, then places the curated labels near it, or in its whole quadrant, again with it. It keeps the result that names the most disagreements, then the most blips, then has the shortest leaders. A blip with no spot that keeps the rules stays unnamed: a wrong name is worse than none, and the comparison and the table name every disagreement.
- **Small scopes name the disagreements.** The builder card, and a shared radar at 640px or less, label only the blips that differ from Konstantin's radar, at 12px or more. A label that finds no spot in its quadrant may sit in a margin of up to four rows below the scope (bottom quadrants) or above it (top quadrants), on its quadrant's side, joined by a leader of at most 16 font sizes. The leader meets the label's near end, or the middle of its near edge when the label sits above or below the blip. In these scopes a leader keeps 3px from every other blip and label and 6px from every other leader. The scope's view box grows to show the margin. When a disagreement is still unnamed, the labels are placed again with the quadrant names free to move: each name then moves out past the labels of its half, and the two names at the top or at the bottom keep one line. The builder card's scope reaches 16px into the card's padding on each side, so its labels have more room.
- **Label sizes.** The overview uses 12.5 scope units (`F_OVER`), the zoomed quadrant 8.5 (`F_ZOOM`) and 14 on phones (`F_ZOOM_M`), so zoomed labels render at 12px or more at 390, 1024 and 1440. The small scopes grow their font in scope units as the scope shrinks, so their labels render at 12px or more.
- **Check it in a browser.** A browser script in the private working repo (`scripts/scope-check.js`) measures a drawn scope: rule breaks (overlaps, labels or leaders over another blip, labels level with another blip, labels nearer another blip, leaders closer than 3px, labels or leaders over a quadrant name), named blips, named disagreements and the smallest label in pixels. Run it with `agent-browser eval "$(sed 's|__SEL__|.b-card svg.radar|' scripts/scope-check.js)"` on the builder after "Edit a copy", with `.deck svg.radar` on a shared radar, or with `[data-radar] svg.radar` on `/radar/`. A pass has no rule breaks. On 17 shared radars with 91 disagreements (12 October 2026), the builder card names 90 at 1440x900 and 87 at 390px, a shared radar names 89 at 390px, and at 1440px it names every disagreement and 680 of 681 blips.

## Colour

Konstantin's feedback on 2026-10-12 replaced the old rule "Colour encodes change, and nothing else":

> "While I like white on a dark background, I find the readability of the page low. This is likely about spacing and pacing."
>
> On the builder's ring control: "This, too, is technically clear, but hard on the eye and not easy to grasp. Thoughtful design uses effects, subtle colors and more to improve usability dramatically."

The rules since then:

1. **Strong colour is for change.** Magenta, blue and orange mark new, moved in and moved out (or, on other radars, disagreement). Nothing else uses them.
2. **Soft colour is for reading.** A colour may appear where it helps people grasp the page faster: the ring tints, the surface layers, hover and selected states. It never competes with a change colour.
3. **The rings share one ordinal ramp.** One teal hue (OKLCH hue 176), strongest at Adopt and faintest at Hold. The ramp fills the scope's bands (`--band-N`), the ring chips (`--rN-bg`), the builder's selected segment, and the rails that join a ring to its entries. Its marks (`--rN`: meter bars, borders) are at least 3:1 on paper in both themes.
4. **Ring tints never sit on a blip.** A blip is ink or a change colour. A ring mark always comes with the ring's name (a chip, a segment label, the scope's ring label), and a change mark always has its own shape. Under deuteranopia the Assess mark and the magenta "new" mark look alike (ΔE 0.8), so these two secondary channels are required, not optional.
5. **Text wears ink tokens.** `--ink` for headings and names, `--body` for prose, `--ink-2` for secondary text, `--muted` for meta and the breadcrumb separator. Never a ring or change colour. A coloured mark sits beside the text instead.
6. **No pure white on near-black.** In dark mode, body text is off-white (`--body`, 12:1) and headings are `--ink` (14.5:1). Secondary text stays at 9:1 or more, so nothing is too dim to read.

### Tokens

Defined once in `assets/site.css` on `:root`, with a dark set applied by `prefers-color-scheme` and by the `data-theme` toggle. The social cards (`src/og.rs`) and the builder's PNG export (`assets/build.js`) copy the dark values.

| Role | Light | Dark | Use |
|---|---|---|---|
| `--paper` | `#eef1f2` | `#0f171d` | Ground: cool chart film / scope |
| `--surface` | `#f9fbfb` | `#161f26` | Raised panels: builder panel, comparison panel, data tag, quadrant buttons |
| `--sunk` | `#e4e9ec` | `#1b262d` | Wells and hover rows: segment track, selector well, current nav item |
| `--ink` | `#0f1b24` | `#e1e7eb` | Headings, names, steady blips |
| `--body` | `#202c34` | `#cbd4da` | Prose (12.6:1 light, 12.0:1 dark) |
| `--ink-2` | `#424f58` | `#adb9c2` | Secondary text (7.4:1, 9.0:1) |
| `--muted` | `#59656e` | `#92a1ab` | Meta, stamps, dates (5.3:1, 6.8:1), label leaders on the scope |
| `--ink-3` | `#87949d` | `#58656f` | Graphics only (ticks, old trails): not for text |
| `--r0..3` | `#005b4b` `#276e5f` `#4b7d70` `#6d8b82` | `#5fb6a1` `#5a9e8d` `#54887a` `#4d7268` | Ring marks, Adopt to Hold |
| `--r0..3-bg` | `#c5e9df` … `#e8f0ed` | `#133e34` … `#19211f` | Ring fills: chips, selected segments |
| `--band-0..3` | `#cdece3` … `#ebf1ef` | `#14362e` … `#161d1b` | Scope bands, centre to edge |
| `--new` | `#c5157a` | `#e0468f` | New entries (chart magenta) |
| `--in` | `#1f5fd0` | `#4a86f0` | Moved towards Adopt |
| `--out` | `#d4600f` | `#e06f22` | Moved towards Hold |

Contrast ratios are WCAG 2 on `--paper`. Ink on every ring fill is 9.5:1 or more. Ring marks are 3:1 or more on paper and on their own fill. In dark mode the ring marks stay calmer than the change colours: lower chroma (0.09 at Adopt against 0.16 to 0.20), Adopt at 7.5:1 and Hold at 3.4:1. A brighter Adopt would outshine every change mark, which breaks rule 1. The rails that join a ring chip to its entries are 2px. Every change colour is 3:1 or more on every band (the lowest is `--out` on the light Adopt band, 3.05:1).

The three change colours were validated as a set with the dataviz palette validator, all pairs, in both modes: lightness band, chroma floor, colour-blind separation (worst ΔE 13.6), normal-vision separation and 3:1 contrast all pass. The ring ramp passes the validator's ordinal checks in both modes (one hue, monotone lightness, steps of ΔL 0.06 or more, light end at 3:1 or more). Re-run both if you change a value. A class `r0`..`r3` on any element sets `--rk` and `--rk-bg` for that ring.

## Type

- **Archivo** (variable, width 62–125%, weight 100–900) carries all words. Page titles, entry names and the wordmark use 125% width and a heavy weight. An entry name over 24 characters takes a smaller size (`h1.long`), so it stays within three lines. Section headings use 112–115%. Body text uses normal width.
- **Martian Mono** carries data only: counts, dates in lists, URLs, code and the scope's labels. At 75% width for radar labels, where label placement assumes an advance of exactly 0.6 em (`MONO_ADVANCE` in `src/layout.rs`). Edition stamps ("Edition 01 · 12 Oct 2026") use Archivo with tabular numbers.
- **Capitals belong to the instrument.** Only the wordmark and the scope's quadrant and ring names are set in capitals. The scope's entry labels keep the entry's own case, so they keep the word shapes of the names in the lists. Labels, chips, table heads, buttons and breadcrumbs are sentence case.
- **Scale** (`--fs-*`): body 18px with line height 1.62; lede 21px; secondary text 16px; small text 14px; the smallest text is 12px (mono stamps). Headings use `--fs-h3`, `--fs-h2` and `--fs-h1`.
- Self-hosted Latin subsets, plus tiny arrow-only subsets for ← and →. Archivo is preloaded.

## Spacing

- One scale, `--s1` to `--s8` (0.25rem to 4rem). Use the tokens. Don't invent gaps.
- **Section rhythm:** `--sect` (2.75rem on phones, 4.25rem at 1440) above and below every section, with a hairline between sections. Inside a section, blocks are `--s5` apart and paragraphs `--s4`.
- **Rules are hairlines.** 1px `--rule` between rows, 1px `--rule-2` under headings. No 2px ink rules: space and surfaces separate blocks.
- **Radii by role:** `--r-s` (3px) for chips, `--r-m` (6px) for buttons, inputs and segments, `--r-l` (10px) for panels.

## Interaction

- Every control has a hover, a visible `:focus-visible` ring (`--focus`, 2px, offset 3px) and a pressed state (a 1–4% press-in). Transitions are 0.16s on colour, background and shadow.
- **The builder's ring control** is a radio group per entry: five labelled segments (ring name, then "Not on my radar" after a divider) on a sunk track. Only the chosen segment, and the one under the pointer or focus, shows its meter; the others are muted words. Hover previews the choice with the ring's tint. The chosen ring fills with its tint, gets a 2px border in its ring mark colour and bold text. The line under the entry name names the curated ring ("Konstantin: Adopt"). Only when the choice differs does the curated segment get a dashed outline and the ring word a dashed underline, and the same line adds ▲ "You rate it higher", ▼ "You rate it lower" or ◆ "Only on your radar". Row height never changes. Each row has one tab stop. Arrow keys move the choice within the row. Home and End jump to its ends.
- **Replacing a radar offers Undo.** "Reset to Konstantin's radar", "Clear all" and "Edit a copy" show an Undo button for 10 seconds. "Edit a copy" asks before it replaces a saved radar of your own that differs from the shared one, keeps the old one under `pr-build-prev`, and starts without the other person's name. A copy of your own radar keeps your name.
- **"Not on my radar" on hover** darkens the well. Only the chosen segment is raised, filled and outlined, so a hover never looks like a choice.
- **Nothing moves when the zoom changes.** A pressed quadrant button changes its border and ink, not its weight. The rail keeps the height of the longest quadrant text, so the ring counts stay in place.
- **The radar's zoom instruction works on every device:** "Select All quadrants to zoom out". Devices with a pointer that hovers also read "or press Escape".
- **Deep links.** `build/#e=<id>` opens the list at that entry's row (entry pages use it). A pasted share link reloads the page; an in-page jump such as `#b-card` does not.
- The share-link format in `assets/build.js` is fixed: links already shared must keep working, and the curated radar moves every week, so a link never stores only the differences from it.

## Motion

One orchestrated moment: on the first visit in a session, a beam sweeps the scope once and each blip pings as it passes. The beam turns its gradient (`@property --sweep`), not its box, so it never pokes out past the page edge on phones. Everything else is feedback: hover, focus, zoom (0.75 s), replay (0.8 s per edition). `prefers-reduced-motion` turns all of it off. The page is complete without any of it.

## Rules for agents changing the design

1. Strong colour is for change. Ring tints and surfaces are soft and never sit on a blip. Don't colour-code quadrants.
2. Text wears ink tokens, never a ring or change colour. A coloured mark sits beside the text instead.
3. Every colour is a token with a light and a dark value. No literal colours in component rules. Check new pairs against WCAG AA (4.5:1 text, 3:1 marks) in both themes.
4. Keep the page complete at rest: nothing hidden behind scroll or JS to be readable.
5. Check `cargo run --release -- build`, then look at the page at 390, 768, 1024 and 1440 px, light and dark. On `/radar/`, zoom each quadrant and confirm the selector stays clear of the scope. On `/build/`, use the ring control with the mouse and with the keyboard.
6. One class name, one meaning. `band` is taken by the scope's ring circles; page sections are `sect`; `r0`..`r3` set the ring context.
7. Copy is plain, specific English: one idea per sentence, active voice, no slogans, no "X isn't Y" contrasts, no rhetorical questions. Nothing on the site is called a draft or a preview. The curated radar is named by its `title` in `data/radars/product-radar.toml` ("Konstantin's radar"); never hard-code that name.
