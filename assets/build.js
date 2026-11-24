/* Build your radar. State lives in the URL hash (shareable) and localStorage (work in progress). */
(() => {
  "use strict";
  const d = document;
  const root = d.querySelector("[data-builder]");
  const cfgEl = d.getElementById("build-data");
  if (!root || !cfgEl) return;
  const cfg = JSON.parse(cfgEl.textContent);
  const esc = s => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  const byId = new Map(cfg.cat.map(c => [c.id, c]));
  const KEY = "pr-build", PREV = "pr-build-prev";
  const meter = k => '<span class="meter" aria-hidden="true">' + [0, 1, 2, 3].map(j => j < 4 - k ? '<i class="on"></i>' : "<i></i>").join("") + "</span>";
  const chip = k => `<span class="ring r${k}">${meter(k)}${esc(cfg.r[k])}</span>`;

  // ---- state ------------------------------------------------------------
  let st = { owner: "", p: {}, x: [] };
  const seed = () => { const p = {}; cfg.cat.forEach(c => { if (c.o >= 0) p[c.id] = c.o; }); return p; };
  const copy = s => ({ owner: s.owner, p: { ...s.p }, x: s.x.map(x => ({ ...x })) });

  // The share-link format is fixed: links already shared must keep working.
  function encode(s) {
    const parts = [];
    if (s.owner) parts.push("n=" + encodeURIComponent(s.owner));
    for (let k = 0; k < 4; k++) {
      const ids = Object.keys(s.p).filter(id => s.p[id] === k);
      if (ids.length) parts.push(k + "=" + ids.join("."));
    }
    if (s.x.length) parts.push("x=" + s.x.map(i => encodeURIComponent(i.n) + "~" + i.q + "~" + i.r).join("|"));
    return parts.join("&");
  }
  function decode(h) {
    const s = { owner: "", p: {}, x: [], e: "" };
    h.replace(/^#/, "").split("&").forEach(kv => {
      const i = kv.indexOf("="); if (i < 0) return;
      const k = kv.slice(0, i), v = kv.slice(i + 1);
      if (k === "n") s.owner = decodeURIComponent(v).slice(0, 40);
      else if (k === "e") s.e = v;
      else if (/^[0-3]$/.test(k)) v.split(".").forEach(id => { if (byId.has(id)) s.p[id] = +k; });
      else if (k === "x") v.split("|").forEach(t => {
        const [n, q, r] = t.split("~");
        if (n && +q >= 0 && +q < 4 && +r >= 0 && +r < 4) s.x.push({ n: decodeURIComponent(n).slice(0, 40), q: +q, r: +r });
      });
    });
    return s;
  }
  // Same placements and same additions, in any order; the owner's name does not count.
  const sameRadar = (a, b) => {
    const ka = Object.keys(a.p).sort(), kb = Object.keys(b.p).sort();
    return ka.length === kb.length && ka.every((k, i) => k === kb[i] && a.p[k] === b.p[k]) && JSON.stringify(a.x) === JSON.stringify(b.x);
  };
  const save = () => { try { localStorage.setItem(KEY, JSON.stringify(st)); } catch (e) { /* private mode */ } };
  const loadSaved = () => {
    let s = null;
    try { s = JSON.parse(localStorage.getItem(KEY) || "null"); } catch (e) { s = null; }
    return s && s.p ? { owner: s.owner || "", p: s.p, x: s.x || [] } : null;
  };

  const hash = location.hash.length > 2 ? decode(location.hash) : null;
  const shared = hash && (Object.keys(hash.p).length || hash.x.length) ? hash : null;
  if (shared) st = { owner: shared.owner, p: shared.p, x: shared.x };
  else st = loadSaved() || { owner: "", p: seed(), x: [] };
  let viewing = !!shared;
  // Pasting another share link into this tab only changes the hash: read it afresh. In-page
  // jumps such as #b-card change the hash too, and stay.
  addEventListener("hashchange", () => { const h = decode(location.hash); if (Object.keys(h.p).length || h.x.length || h.e) location.reload(); });

  // ---- elements ---------------------------------------------------------
  const rows = [...root.querySelectorAll(".b-q li[data-id]")];
  const owner = d.getElementById("b-owner");
  const stats = root.querySelector(".b-stats");
  const msgs = [...root.querySelectorAll(".b-msg")];
  const barN = root.querySelector(".b-bar-n");
  const toml = d.getElementById("b-toml");
  const fig = root.querySelector(".b-scope");
  const svg = fig.querySelector("svg");
  const gBlips = svg.querySelector(".blips"), gLbl = svg.querySelector(".lbl");
  const diffHead = root.querySelector(".b-dh"), diffList = root.querySelector(".b-dl"), leftLine = root.querySelector(".b-left");

  // A status line, with an Undo button for 10 seconds when the action replaced the radar.
  function say(text, undo, ms = undo ? 10000 : 4000) {
    clearTimeout(say.t);
    msgs.forEach(m => {
      m.textContent = text;
      if (undo) {
        const b = d.createElement("button");
        b.type = "button"; b.className = "b-undo"; b.textContent = "Undo";
        b.addEventListener("click", () => { clearTimeout(say.t); undo(); });
        m.append(" ", b);
      }
    });
    say.t = setTimeout(() => msgs.forEach(m => { m.textContent = ""; }), ms);
  }

  // ---- layout (mirrors src/layout.rs) -------------------------------------
  const OUTER = cfg.ro, START = { br: 0, bl: 90, tl: 180, tr: 270 };
  const F = cfg.fo, ASC = 1.05, DESC = 0.24, LINE = ASC + DESC, ADV = 0.6;
  const band = k => [k ? OUTER[k - 1] + 4 : 42, OUTER[k] - 4];
  const rad = i => i.cls === "s-new" ? 15 : 10.25;
  const kept = it => it.p && it.k === it.o;
  // A blip in the curated ring keeps its place on the curated scope (cfg.cat[].p), so a radar
  // started from it looks like it, and its labels fit as they do there. Every other blip takes
  // a free point of its ring cell, clear of the axes: the first point 30 units or more from
  // every placed blip where its own label finds a spot, freest first (farthest from the placed
  // blips, counting up to 60 units, then nearest the middle of the cell).
  // A moved blip also keeps clear of the curated labels of the blips that stayed, so those
  // labels stay where Konstantin's radar has them. `font` is the label size the scope will
  // show: the builder card labels only the disagreements, larger, and draws no curated label.
  // There a moved blip keeps clear of the labels of the moved blips placed before it.
  function layout(items, font = F) {
    const pos = new Map(), pad = 8 * Math.PI / 180, span = Math.PI / 2 - 2 * pad;
    const placed = items.filter(kept);
    placed.forEach(it => pos.set(it.key, [it.p[0], it.p[1], it.q]));
    const pre = font !== F ? [] : placed.filter(it => byId.has(it.key) && byId.get(it.key).L).map(it => {
      const [x, y, a, lines, lead] = byId.get(it.key).L;
      return { key: it.key, b: blockBox(x, y, a, lines, F), level: false, lead: lead || null };
    });
    const rest = items.filter(it => !pos.has(it.key)).sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
    for (const it of rest) {
      const [a, b] = band(it.k), t0 = START[cfg.c[it.q]] * Math.PI / 180 + pad, pts = [];
      for (let i = 0; i <= 24; i++) for (let j = 0; j <= 5; j++) {
        const th = t0 + span * i / 24, r = a + (b - a) * j / 5, x = r * Math.cos(th), y = r * Math.sin(th);
        if (Math.abs(x) < 12.25 || Math.abs(y) < 12.25) continue;
        let dd = 60;
        pos.forEach(([px, py]) => { dd = Math.min(dd, Math.hypot(px - x, py - y)); });
        pts.push([dd - 0.01 * Math.hypot(i - 12, (j - 2.5) * 4), dd, [x, y, it.q]]);
      }
      pts.sort((p, q) => q[0] - p[0]);
      placed.push(it);
      const clear = ([x, y]) => pre.every(l => bdist(l.b, x, y) - rad(it) >= font && !levelAtEnd(l.b, x, y, rad(it), font) && (!l.lead || segDist(l.lead, x, y) >= rad(it) + gapU(font)));
      let got = null;
      const fits = ([, dd, p]) => { if (dd < 30 || !clear(p)) return false; pos.set(it.key, p); [got] = labels([it], placed, pos, font, pre); return !!got; };
      const loose = ([, dd, p]) => { if (dd < 30) return false; pos.set(it.key, p); return labels([it], placed, pos, font).length > 0; };
      const at = pts.find(fits);
      pos.set(it.key, (at || pts.find(loose) || pts[0])[2]);
      // The small scopes label only the blips that moved: each one's label keeps the spots of
      // those placed before it free.
      if (at && font !== F) pre.push(got);
    }
    return pos;
  }

  // Geometry shared by the label passes.
  const hit = (a, b, gx = 0, gy = 0) => a[0] - gx < b[2] && b[0] < a[2] + gx && a[1] - gy < b[3] && b[1] < a[3] + gy;
  const bdist = (b, x, y) => Math.hypot(Math.max(b[0] - x, 0, x - b[2]), Math.max(b[1] - y, 0, y - b[3]));
  const segDist = (s, x, y) => { const dx = s[2] - s[0], dy = s[3] - s[1], l = dx * dx + dy * dy, t = l ? Math.max(0, Math.min(1, ((x - s[0]) * dx + (y - s[1]) * dy) / l)) : 0; return Math.hypot(s[0] + t * dx - x, s[1] + t * dy - y); };
  // A dot level with a label and close beyond one of its ends reads as the label's own
  // ("Guard hooks ●"): no other blip sits there (layout::level_at_end, END_CLEAR 3 font sizes).
  // Distance from a blip's edge to the nearest attachment point of a label box (layout::attach_dist).
  const attach = (b, x, y, r) => { const cx = (b[0] + b[2]) / 2, cy = (b[1] + b[3]) / 2; return Math.min(...[[b[0], cy], [b[2], cy], [cx, b[1]], [cx, b[3]], [b[0], b[1]], [b[2], b[1]], [b[0], b[3]], [b[2], b[3]]].map(([px, py]) => Math.hypot(px - x, py - y))) - r; };
  const levelAtEnd = (b, x, y, r, font) => y > b[1] - 0.25 * font && y < b[3] + 0.25 * font && Math.max(b[0] - x, x - b[2]) - r < 3 * font;
  const segsDist = (a, b) => {
    const cr = (p, q, r) => (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
    const a0 = [a[0], a[1]], a1 = [a[2], a[3]], b0 = [b[0], b[1]], b1 = [b[2], b[3]];
    if (Math.sign(cr(a0, a1, b0)) * Math.sign(cr(a0, a1, b1)) < 0 && Math.sign(cr(b0, b1, a0)) * Math.sign(cr(b0, b1, a1)) < 0) return 0;
    return Math.min(segDist(a, b[0], b[1]), segDist(a, b[2], b[3]), segDist(b, a[0], a[1]), segDist(b, a[2], a[3]));
  };
  // Space a leader keeps from other blips, labels and leaders: 3 units on the curated scope,
  // which draws about one unit per pixel; 3px in the small scopes, where the font is 12px.
  const gapU = font => font === F ? 3 : Math.max(3, font / 4);
  const segHits = (s, b) => { for (let k = 0; k <= 24; k++) { const x = s[0] + (s[2] - s[0]) * k / 24, y = s[1] + (s[3] - s[1]) * k / 24; if (x > b[0] && x < b[2] && y > b[1] && y < b[3]) return true; } return false; };
  // Splits a long name at the space closest to its middle (layout::wrap2).
  const wrap2 = t => {
    if (t.length < 14) return null;
    let cut = -1;
    for (let i = 0; i < t.length; i++) if (t[i] === " " && (cut < 0 || Math.abs(i - t.length / 2) < Math.abs(cut - t.length / 2))) cut = i;
    return cut < 0 ? null : [t.slice(0, cut), t.slice(cut + 1)];
  };
  const blockBox = (x, y, a, lines, font) => { const w = Math.max(...lines.map(l => l.length)) * font * ADV, x0 = a === "end" ? x - w : a === "middle" ? x - w / 2 : x; return [x0, y - ASC * font, x0 + w, y + ((lines.length - 1) * LINE + DESC) * font]; };
  // Ring names on the horizontal axis (site.css .rlab), as in layout::axis_marks.
  const marks = [0, 1, 2, 3].flatMap(k => { const m = ((k ? OUTER[k - 1] : 0) + OUTER[k]) / 2, w = 6 * 10 * 0.79; return [m, -m].map(c => [c - w / 2 - 1.5, -6 - 10.5 - 1, c + w / 2 + 1.5, -6 + 2.4 + 1]); });
  const quadBounds = (q, font) => { const sx = cfg.c[q][1] === "l" ? -1 : 1, sy = cfg.c[q][0] === "t" ? -1 : 1; return [sx < 0 ? -398 : font, sy < 0 ? -354 : 1, sx < 0 ? -font : 398, sy < 0 ? -1 : 354]; };
  // The small scopes (the builder card, a shared radar on a phone) label only the disagreements,
  // at 12px or more. A label that finds no spot inside its quadrant may sit in a margin of up to
  // four rows of labels above (top quadrants) or below (bottom quadrants) the scope, under its
  // quadrant's name. The scope's view box then grows to show it (`fitView`).
  const MARGIN = 4 * (ASC + DESC) + 3 * 0.6 + 0.2;
  // The quadrant names in the corners, in scope units, measured as drawn (4 units of space around).
  const qlabBoxes = () => [...svg.querySelectorAll(".qlab text")].map(t => {
    let b = null;
    try { b = t.getBBox(); } catch (e) { b = null; }
    return b && b.width ? [b.x - 4, b.y - 4, b.x + b.width + 4, b.y + b.height + 4] : null;
  }).filter(Boolean);

  // Same rules as src/layout.rs (see "Scope labels" in docs/design.md): em box 1.05 above and
  // 0.24 below the baseline, 0.6em per glyph. A label stays in its quadrant, clear of every
  // blip and of the ring names. Labels on one line keep two font sizes apart. Stacked labels
  // keep 0.6 font sizes apart when both sit level with their blips, else one font size. Every
  // other labelled blip is at least one font size further from the label's attachment points
  // than its own blip, and none sits between the label and its blip. A short leader joins a
  // label that sits away from its blip. `pre` holds labels already placed (see `fixedLabels`).
  // Two orders run; the one that shows more labels wins. A label without a spot is left out
  // here; `labelAll` then finds it one.
  function labels(items, all, pos, font, pre = []) {
    const h = font * LINE;
    const dots = all.map(i => { const [x, y] = pos.get(i.key); return { key: i.key, x, y, r: rad(i) }; });
    const labelled = new Set([...items.map(i => i.key), ...pre.map(p => p.key)]);
    const obs = [...dots.map(o => [o.x - o.r, o.y - o.r, o.x + o.r, o.y + o.r]), ...marks];
    // The strip between a label and its blip, 2.5 font sizes deep, towards the blip.
    const facing = (b, x, y) => { const dd = 2.5 * font, dx = Math.max(b[0] - x, x - b[2]), dy = Math.max(b[1] - y, y - b[3]); return dy >= dx ? (y > b[3] ? [b[0], b[1], b[2], b[3] + dd] : [b[0], b[1] - dd, b[2], b[3]]) : (x < b[0] ? [b[0] - dd, b[1], b[2], b[3]] : [b[0], b[1], b[2] + dd, b[3]]); };
    function spots(it) {
      const [x, y, q] = pos.get(it.key), right = x >= 0, r = rad(it), off = r + 0.3 * font, s = right ? 1 : -1;
      const bounds = quadBounds(q, font), cs = [];
      // Beside the blip on one line, then on two lines when the name is long (as wrap2 in layout.rs).
      for (const lines of [[it.label], wrap2(it.label)].filter(Boolean)) {
        const n = lines.length, w = Math.max(...lines.map(l => l.length)) * font * ADV, hb = ((n - 1) * LINE + ASC + DESC) * font;
        const box = (ax, by, a, level, lead) => { const x0 = a === "end" ? ax - w : a === "middle" ? ax - w / 2 : ax; return { b: [x0, by - ASC * font, x0 + w, by + ((n - 1) * LINE + DESC) * font], x: ax, y: by, a, level, lead, lines }; };
        const mid = y - hb / 2 + ASC * font, step = 0.55 * h;
        const side = (rt, dy) => box(rt ? x + off : x - off, mid + dy, rt ? "start" : "end", dy === 0);
        const vert = (up, k) => box(x + k, up ? y - r - 0.5 - hb + ASC * font : y + r + 0.5 + ASC * font, "middle", false);
        const corner = (dx, up) => box(x + dx * r * 0.7, up ? y - r * 0.7 - hb + ASC * font : y + r * 0.7 + ASC * font, dx > 0 ? "start" : "end", false);
        cs.push(side(right, 0), side(!right, 0), vert(true, 0), vert(false, 0), side(right, -step), side(!right, -step), side(right, step), side(!right, step),
          corner(s, true), corner(s, false), corner(-s, true), corner(-s, false));
        for (const k of [0.3, 0.45]) for (const up of [true, false]) cs.push(vert(up, s * w * k), vert(up, -s * w * k));
        if (n > 1) continue;
        // Leader spots, one line, shortest leader first.
        const leads = [];
        for (const dd of [1.4, 2.2, 3.0, 3.8, 4.8]) for (const j of [0, -1, 1, -2, 2]) for (const rt of [right, !right]) leads.push([x + (rt ? 1 : -1) * (r + dd * font), y + j * 0.75 * h, rt]);
        leads.sort((a, b) => Math.hypot(a[0] - x, a[1] - y) - Math.hypot(b[0] - x, b[1] - y));
        for (const [px, py, rt] of leads) {
          const ex = px + (rt ? -2 : 2), len = Math.hypot(ex - x, py - y);
          cs.push(box(px, py + 0.405 * font, rt ? "start" : "end", true, [x + (ex - x) / len * r, y + (py - y) / len * r, ex, py]));
        }
      }
      const free = c => { const b = c.b; return b[0] >= bounds[0] && b[2] <= bounds[2] && b[1] >= bounds[1] && b[3] <= bounds[3] && !obs.some(o => hit(o, b)); };
      // Only labelled blips compete for a label: when the builder labels only disagreements,
      // those blips wear a change colour and shape, and an ink blip never has a label. Every
      // blip keeps clear of the label and of its leader.
      const own = c => {
        const g = c.lead ? 0 : attach(c.b, x, y, r), z = facing(c.b, x, y);
        return dots.every(o => o.key === it.key || (bdist(c.b, o.x, o.y) - o.r >= 0.5 * font && !levelAtEnd(c.b, o.x, o.y, o.r, font) && (!c.lead || segDist(c.lead, o.x, o.y) >= o.r + gapU(font)) &&
          (!labelled.has(o.key) || (attach(c.b, o.x, o.y, o.r) >= g + font && (c.lead || bdist(z, o.x, o.y) >= o.r)))));
      };
      return cs.filter(c => free(c) && own(c));
    }
    const sp = new Map(items.map(it => [it.key, spots(it)]));
    const clash = (a, b) => hit(a.b, b.b, 2 * font, (a.level && b.level ? 0.6 : 1) * font) || (a.lead && segHits(a.lead, b.b)) || (b.lead && segHits(b.lead, a.b));
    const run = order => {
      const placed = [...pre], out = [];
      for (const it of order) {
        const c = sp.get(it.key).find(c => !placed.some(p => clash(p, c)));
        if (!c) continue;
        placed.push(c);
        out.push({ key: it.key, x: c.x, y: c.y, a: c.a, lines: c.lines, b: c.b, level: c.level, cls: it.cls, lead: c.lead });
      }
      return out;
    };
    // Disagreements come first in both orders; the second order starts with the most crowded.
    const steady = it => it.cls === "s-steady" ? 1 : 0, score = ls => ls.filter(l => l.cls !== "s-steady").length * 1000 + ls.length;
    const first = run(items), fewest = run([...items].sort((a, b) => steady(a) - steady(b) || sp.get(a.key).length - sp.get(b.key).length));
    return score(fewest) > score(first) ? fewest : first;
  }

  // The curated scope's own labels (cfg.cat[].L, from src/svg.rs) for blips that keep their
  // curated place, unless a blip that moved now sits close to the label or its leader.
  function fixedLabels(items, all, pos, font, drop) {
    const moved = all.filter(i => !kept(i)).map(i => { const [x, y] = pos.get(i.key); return [x, y, rad(i)]; });
    const keep = [], rest = [];
    for (const it of items) {
      const L = byId.get(it.key) && byId.get(it.key).L;
      if (font === F && L && kept(it) && !drop.has(it.key)) {
        const [x, y, a, lines, lead] = L, b = blockBox(x, y, a, lines, font);
        if (moved.every(([mx, my, mr]) => bdist(b, mx, my) - mr >= font && !levelAtEnd(b, mx, my, mr, font) && (!lead || segDist(lead, mx, my) >= mr + 3))) {
          keep.push({ key: it.key, x, y, a, lines, b, level: false, cls: it.cls, lead: lead || null });
          continue;
        }
      }
      rest.push(it);
    }
    return [keep, rest];
  }

  // Every item gets a label: the curated labels first, then the rules above, then the nearest
  // free spot in the blip's quadrant, joined to the blip by a leader. That spot follows the
  // leader rules of src/layout.rs: the leader keeps 3 units (3px in the small scopes) clear of
  // every other blip, label and leader; no other blip touches the label, sits level with it
  // beyond its ends, or comes within one font size of its attachment points, and the leader is
  // at most 8 font sizes long (14 for a disagreement, 16 in the small scopes).
  // In the small scopes a label may also sit in the margin above or below the scope (`MARGIN`).
  // Disagreements look first. A blip with no such spot stays unnamed: a wrong name is worse
  // than none, and the table below the scope names it.
  // `freeQ`: the small scope may move its quadrant names outwards, past the labels (`fitView`),
  // so labels need not keep clear of them.
  function labelAll(items, its, pos, font, drop = new Set(), freeQ = false) {
    const wide = font !== F, qb = wide && !freeQ ? qlabBoxes() : [], G = gapU(font);
    const [fixed, rest] = fixedLabels(items, its, pos, font, drop);
    const first = [...fixed, ...labels(rest, its, pos, font, fixed)];
    const dots = its.map(i => { const [x, y] = pos.get(i.key); return [x, y, rad(i), i.key]; });
    const grow = (b, g) => [b[0] - g, b[1] - g, b[2] + g, b[3] + g];
    const order = rest.filter(it => !first.some(l => l.key === it.key)).sort((a, b) => (a.cls === "s-steady") - (b.cls === "s-steady"));
    // Every spot of one label that keeps clear of the blips and the scope's furniture, shortest
    // leader first. Labels placed before it are checked in `fill`. One render works the spots
    // out once (`spotMemo`); the quadrant names are checked last, as `freeQ` may ignore them.
    const spots = it => {
      const k = it.key + "|" + font, raw = (spotMemo && spotMemo.get(k)) || rawSpots(it);
      if (spotMemo) spotMemo.set(k, raw);
      return qb.length ? raw.filter(c => !qb.some(mk => hit(mk, c.b) || (c.lead && segHits(c.lead, mk)))) : raw;
    };
    function rawSpots(it) {
      const [x, y, q] = pos.get(it.key), r = rad(it), sx = cfg.c[q][1] === "l" ? -1 : 1, sy = cfg.c[q][0] === "t" ? -1 : 1, m = MARGIN * font;
      // Its quadrant (in the small scopes out to the corners), and below or above that the margin
      // under the quadrant's name, so a name in the margin sits with its quadrant's.
      const areas = wide ? [[sx < 0 ? -398 : font, sy < 0 ? -398 : 1, sx < 0 ? -font : 398, sy < 0 ? -1 : 398], [sx < 0 ? -398 : font, sy < 0 ? -400 - m : 404, sx < 0 ? -font : 398, sy < 0 ? -404 : 400 + m]] : [quadBounds(q, font)];
      const others = dots.filter(d => d[3] !== it.key), cap = (it.cls === "s-steady" ? 8 : wide ? 16 : 14) * font, out = [];
      for (const lines of [[it.label], wrap2(it.label)].filter(Boolean)) {
        const w = Math.max(...lines.map(l => l.length)) * font * ADV, down = ((lines.length - 1) * LINE + DESC) * font;
        for (const bounds of areas) for (let gx = bounds[0]; gx <= bounds[2]; gx += font / 2) for (let gy = bounds[1] + ASC * font; gy <= bounds[3] - down; gy += font / 2) {
          for (const a of ["start", "end"]) {
            const x0 = a === "start" ? gx : gx - w, b = [x0, gy - ASC * font, x0 + w, gy + down];
            if (b[0] < bounds[0] || b[2] > bounds[2] || bdist(b, x, y) < r + 2) continue;
            if (others.some(([ox, oy, or]) => bdist(b, ox, oy) - or < 0.5 * font || levelAtEnd(b, ox, oy, or, font))) continue;
            if (marks.some(mk => hit(mk, b))) continue;
            // The leader meets the label at its near end, level with the text, or, for a label
            // above or below its blip, on the near edge, straight under or over the blip where
            // the label spans it.
            const ends = [[a === "start" ? b[0] - 2 : b[2] + 2, gy - (ASC - DESC) / 2 * font]];
            if (y < b[1] || y > b[3]) ends.push([Math.max(b[0] + 4, Math.min(b[2] - 4, x)), y < b[1] ? b[1] - 2 : b[3] + 2]);
            const near = bdist(b, x, y) - r < 0.4 * font;
            for (const [ex, ey] of near ? ends.slice(0, 1) : ends) {
              const len = Math.hypot(ex - x, ey - y);
              if (len > r + cap) continue;
              const lead = near || !len ? null : [x + (ex - x) / len * r, y + (ey - y) / len * r, ex, ey];
              if (lead) {
                if (segHits(lead, b) || others.some(([ox, oy, or]) => segDist(lead, ox, oy) < or + G || attach(b, ox, oy, or) < font)) continue;
              } else {
                const g = attach(b, x, y, r);
                if (others.some(([ox, oy, or]) => attach(b, ox, oy, or) < g + font)) continue;
              }
              out.push({ key: it.key, x: a === "start" ? x0 : x0 + w, y: gy, a, lines, b, level: false, cls: it.cls, lead, len });
            }
          }
        }
      }
      return out.sort((p, q) => p.len - q.len);
    }
    // Every label but the curated ones may also start from scratch, so a spot the first pass
    // took can go to a label that has no other. Its spots are worked out when a step first needs them.
    const memo = new Map(), byKey = new Map(items.map(it => [it.key, it])), mine = new Set(rest.map(it => it.key));
    const cands = { has: k => mine.has(k), get: k => { if (!memo.has(k)) memo.set(k, spots(byKey.get(k))); return memo.get(k); } };
    // Two leaders keep 3 units apart, and 6px in the small scopes, so they never read as one line.
    const GL = wide ? font / 2 : 3;
    const clash = (p, c) => hit(p.b, c.b, 2 * font, 0.6 * font) || (p.lead && segHits(p.lead, grow(c.b, G))) || (c.lead && (segHits(c.lead, grow(p.b, G)) || (p.lead && segsDist(c.lead, p.lead) < GL)));
    const fill = (ord, start = first) => {
      const placed = [...start];
      for (const it of ord) { if (placed.some(l => l.key === it.key)) continue; const c = cands.get(it.key).find(c => !placed.some(p => clash(p, c))); if (c) placed.push(c); }
      return placed;
    };
    const named = ls => ls.filter(l => l.cls !== "s-steady").length;
    const leadLen = ls => ls.reduce((t, l) => t + (l.lead ? Math.hypot(l.lead[2] - l.lead[0], l.lead[3] - l.lead[1]) : 0), 0);
    const better = (a, b) => named(a) > named(b) || (named(a) === named(b) && (a.length > b.length || (a.length === b.length && leadLen(a) < leadLen(b))));
    // Disagreements first, then the blips in the same ring.
    const unnamed = ls => items.filter(it => !ls.some(l => l.key === it.key)).sort((a, b) => (a.cls === "s-steady") - (b.cls === "s-steady"));
    let placed = fill(order);
    // When a blip is still unnamed, try other orders: the blips nearest the top or bottom first,
    // left to right, right to left, fewest spots first, then shuffled orders (the same ones on
    // every run: a fixed seed). Keep the order that names the most disagreements, then the most
    // blips, then has the shortest leaders.
    if (unnamed(placed).length) {
      const by = (list, f) => [...list].sort((a, b) => (a.cls === "s-steady") - (b.cls === "s-steady") || f(a) - f(b));
      const keys = [it => -Math.abs(pos.get(it.key)[1]), it => pos.get(it.key)[0], it => -pos.get(it.key)[0], it => cands.get(it.key).length];
      // The small scopes also place again the labels the first pass found; the curated scope
      // keeps them (there are many, and it has the curated labels to free instead, below).
      const lists = wide ? [[order, first], [rest, fixed]] : [[order, first]];
      for (const f of keys) for (const [list, start] of lists) { const again = fill(by(list, f), start); if (better(again, placed)) placed = again; }
      let seed = 7;
      const rnd = () => { seed = (seed * 16807) % 2147483647; return seed / 2147483647; };
      const [pool, base] = lists[lists.length - 1];
      for (let k = 0; k < 24 && unnamed(placed).length; k++) {
        const ord = pool.map(it => [rnd(), it]).sort((a, b) => a[0] - b[0]).map(p => p[1]);
        const again = fill(by(ord, () => 0), base);
        if (better(again, placed)) placed = again;
      }
    }
    // Then move labels out of the way: an unnamed blip takes a spot that clashes with one or two
    // placed labels (not curated ones), when those find other spots.
    for (const u of unnamed(placed)) {
      if (!cands.has(u.key)) continue;
      for (const c of cands.get(u.key)) {
        const hits = placed.filter(p => clash(p, c));
        if (hits.length > 2 || hits.some(h => !cands.has(h.key))) continue;
        const moved = (hits.length > 1 ? [hits, [hits[1], hits[0]]] : [hits]).map(hs => fill(hs.map(h => byKey.get(h.key)), placed.filter(p => !hits.includes(p)).concat([c])));
        const ok = moved.find(m => m.length === placed.length + 1);
        if (ok) { placed = ok; break; }
      }
    }
    // A blip left unnamed: free the curated labels around it, then those of its whole quadrant,
    // and place them again with it, disagreements first. Keep a result only when it is better.
    const missing = unnamed(placed);
    if (missing.length && !drop.size && fixed.length) {
      let best = placed;
      for (const zone of [l => missing.some(m => { const [mx, my] = pos.get(m.key); return bdist(l.b, mx, my) < 8 * font; }), l => missing.some(m => byId.get(l.key).q === m.q)]) {
        const free = new Set(fixed.filter(zone).map(l => l.key));
        if (!free.size) continue;
        const again = labelAll(items, its, pos, font, free, freeQ);
        if (better(again, best)) best = again;
        if (!unnamed(best).length) break;
      }
      return best;
    }
    return placed;
  }

  // The band under a label's middle (4 outside the scope): its halo takes that band's colour.
  const halo = b => { const r = Math.hypot((b[0] + b[2]) / 2, (b[1] + b[3]) / 2), k = OUTER.findIndex(o => r <= o); return k < 0 ? 4 : k; };
  // data-i joins a label and its leader to the blip's index, as in src/svg.rs (scripts/scope-check.js reads it).
  const drawLabels = (ls, font, idx) =>
    ls.filter(l => l.lead).map(l => `<path class="lead" data-i="${idx.get(l.key)}" d="M${l.lead[0].toFixed(1)} ${l.lead[1].toFixed(1)}L${l.lead[2].toFixed(1)} ${l.lead[3].toFixed(1)}"/>`).join("") +
    ls.map(l => `<text class="h${halo(l.b)}" data-i="${idx.get(l.key)}" x="${l.x.toFixed(1)}" y="${l.y.toFixed(1)}" text-anchor="${l.a}">` +
      (l.lines.length === 1 ? esc(l.lines[0]) : l.lines.map((t, k) => `<tspan x="${l.x.toFixed(1)}" y="${(l.y + k * LINE * font).toFixed(1)}">${esc(t)}</tspan>`).join("")) + "</text>").join("");

  // Which labels the scope shows. A shared radar on a screen wider than 640px names every blip
  // at the curated scope's size. The builder card, and a shared radar on a phone, name only the
  // disagreements, at 12px or more: the font grows (in scope units) as the scope shrinks. A
  // resize that changes the choice or the size lays the labels out again.
  // The scope's view box is 800 units wide (layout::VB).
  const phone = matchMedia("(max-width:640px)");
  const editFont = () => { const w = fig.getBoundingClientRect().width; return w ? Math.max(19, Math.ceil(12 * 800 / w)) : 26; };
  const labelFont = () => viewing && !phone.matches ? F : editFont();
  let shownF = 0, resizing = 0;
  addEventListener("resize", () => {
    cancelAnimationFrame(resizing);
    resizing = requestAnimationFrame(() => { if (labelFont() !== shownF) render(); });
  });
  // The view box covers the scope and every label, so a label in the margin below or above
  // the scope stays in view (the figure takes its height from the view box).
  // With `freeQ`, the quadrant names move out past the labels of their half of the scope, so
  // they stay the outermost lines. The two names at the top, or at the bottom, keep one line.
  let spotMemo = null;
  const fitView = (ls, freeQ) => {
    let y0 = -400, y1 = 400;
    const qs = [...svg.querySelectorAll(".qlab")];
    qs.forEach(g => g.removeAttribute("transform"));
    ls.forEach(l => { y0 = Math.min(y0, l.b[1] - 6); y1 = Math.max(y1, l.b[3] + 6); });
    const qb = freeQ ? qlabBoxes() : [];
    for (const down of [false, true]) {
      const side = qb.map((q, i) => [q, qs[i]]).filter(([q]) => (q[1] > 0) === down);
      const need = side.map(([q]) => {
        const near = ls.filter(l => (q[0] + q[2] > 0 ? l.b[2] > 0 : l.b[0] < 0) && (down ? l.b[3] > q[1] - 8 : l.b[1] < q[3] + 8));
        return !near.length ? 0 : down ? Math.max(...near.map(l => l.b[3])) + 8 - q[1] : Math.min(...near.map(l => l.b[1])) - 8 - q[3];
      });
      const dy = down ? Math.max(0, ...need) : Math.min(0, ...need);
      if (!dy) continue;
      side.forEach(([q, g]) => { g.setAttribute("transform", `translate(0 ${dy.toFixed(1)})`); y0 = Math.min(y0, q[1] + dy - 2); y1 = Math.max(y1, q[3] + dy + 2); });
    }
    svg.setAttribute("viewBox", `-400 ${y0.toFixed(1)} 800 ${(y1 - y0).toFixed(1)}`);
  };

  // Colour encodes disagreement with the curated radar (cfg.name): the same vocabulary as movement.
  function items() {
    const out = [];
    for (const id in st.p) {
      const c = byId.get(id); if (!c) continue;
      const k = st.p[id];
      const cls = c.o < 0 ? "s-new" : k < c.o ? "s-in" : k > c.o ? "s-out" : "s-steady";
      out.push({ key: id, name: c.n, label: c.l || c.n, q: c.q, k, o: c.o, p: c.p, cls });
    }
    st.x.forEach((x, i) => out.push({ key: "x" + i, name: x.n, label: x.n, q: x.q, k: x.r, o: -1, cls: "s-new" }));
    return out;
  }

  // Comparison with the curated radar: same ring, higher, lower, only on this radar, left off.
  function compare() {
    let same = 0, both = 0, hi = 0, lo = 0, only = st.x.length;
    const diffs = [], left = [];
    for (const id in st.p) {
      const c = byId.get(id); if (!c) continue;
      if (c.o < 0) { only++; continue; }
      both++;
      if (st.p[id] === c.o) same++;
      else { if (st.p[id] < c.o) hi++; else lo++; diffs.push([Math.abs(st.p[id] - c.o), c, st.p[id]]); }
    }
    cfg.cat.forEach(c => { if (c.o >= 0 && !(c.id in st.p)) left.push(c); });
    diffs.sort((a, b) => b[0] - a[0] || a[1].n.localeCompare(b[1].n));
    left.sort((a, b) => a.n.localeCompare(b.n));
    return { n: Object.keys(st.p).filter(id => byId.has(id)).length + st.x.length, same, both, hi, lo, only, diffs, left };
  }
  const leftText = left => left.length ? `Left off: ${left.slice(0, 5).map(c => esc(c.n)).join(", ")}${left.length > 5 ? ` and ${left.length - 5} more` : ""}.` : "";
  const diffItems = (diffs, whose) => diffs.slice(0, 5).map(([, c, k]) => `<li class="mv ${k < c.o ? "s-in" : "s-out"}"><span class="mk ${k < c.o ? "s-in" : "s-out"}" aria-hidden="true"></span><span class="mv-main"><a href="${esc(cfg.entry + c.id + "/")}">${esc(c.n)}</a><span class="mv-rings">${whose}: ${esc(cfg.r[k])} · ${esc(cfg.who)}: ${esc(cfg.r[c.o])}</span></span></li>`).join("");

  // One tab stop per row: the chosen ring. Arrow keys move within the row.
  const check = (li, k) => li.querySelectorAll(".seg button").forEach(b => {
    const on = +b.dataset.k === k;
    b.setAttribute("aria-checked", String(on));
    b.tabIndex = on ? 0 : -1;
  });
  const diffNote = (k, o) => k < 0 ? "" : o < 0 ? '<i class="mk s-new"></i>Only on your radar' : k < o ? '<i class="mk s-in"></i>You rate it higher' : k > o ? '<i class="mk s-out"></i>You rate it lower' : "";

  function render() {
    rows.forEach(li => {
      const k = li.dataset.id in st.p ? st.p[li.dataset.id] : -1, o = byId.get(li.dataset.id).o;
      li.classList.toggle("set", k >= 0);
      li.classList.toggle("diff", o >= 0 && k !== o);
      check(li, k);
      li.querySelector(".b-diff").innerHTML = diffNote(k, o);
    });
    renderCustom();
    const font = shownF = labelFont(), all = viewing && !phone.matches;
    const its = items(), pos = layout(its, font);
    const idx = new Map(its.map((i, n) => [i.key, n]));
    gBlips.innerHTML = its.map((i, n) => {
      const [x, y] = pos.get(i.key), ours = i.o >= 0 ? `${cfg.who}: ${cfg.r[i.o]}` : `Not on ${cfg.name}`;
      return `<g class="blip ${i.cls}" data-i="${n}" style="transform:translate(${x.toFixed(1)}px,${y.toFixed(1)}px)"><title>${esc(i.name)}: ${esc(cfg.r[i.k])}. ${esc(ours)}.</title><circle class="halo" r="13"/><circle class="dot" r="8"/></g>`;
    }).join("");
    gLbl.classList.toggle("all", all);
    gLbl.style.setProperty("--lf", font + "px");
    const want = all ? its : its.filter(i => i.cls !== "s-steady"), named = ls => ls.filter(l => l.cls !== "s-steady").length;
    svg.querySelectorAll(".qlab").forEach(g => g.removeAttribute("transform"));
    spotMemo = new Map();
    let ls = labelAll(want, its, pos, font), freeQ = false;
    // A disagreement still unnamed in a small scope: try again with the quadrant names free to move.
    if (!all && named(ls) < want.length) { const again = labelAll(want, its, pos, font, new Set(), true); if (named(again) > named(ls)) { ls = again; freeQ = true; } }
    spotMemo = null;
    gLbl.innerHTML = drawLabels(ls, font, idx);
    fitView(ls, freeQ);

    const cmp = compare();
    toml.value = toToml();
    if (viewing) { renderShared(its, cmp); return; }
    // The comparison as a row of count cells, in the same order every time.
    const cell = (n, label, mk) => `<div><dt>${mk ? `<i class="mk ${mk}" aria-hidden="true"></i>` : ""}${label}</dt><dd${n ? "" : ' class="z"'}>${n}</dd></div>`;
    stats.innerHTML = cmp.n
      ? `<p><b>${cmp.n}</b> placed. Compared with ${esc(cfg.name)}:</p><dl class="b-cells">${cell(cmp.same, "Same ring")}${cell(cmp.hi, "Higher", "s-in")}${cell(cmp.lo, "Lower", "s-out")}${cell(cmp.only, "Only yours", "s-new")}${cell(cmp.left.length, "Left off")}</dl>`
      : "<p>Nothing placed yet. Pick a ring for anything you use.</p>";
    barN.innerHTML = `<b>${cmp.n}</b> placed · <span class="nowrap"><i class="mk s-in"></i>${cmp.hi} <i class="mk s-out"></i>${cmp.lo} <i class="mk s-new"></i>${cmp.only}</span>`;
    diffList.innerHTML = diffItems(cmp.diffs, "Yours");
    diffHead.hidden = !cmp.diffs.length;
    leftLine.hidden = !cmp.left.length || !cmp.n;
    leftLine.innerHTML = leftText(cmp.left);
    save();
  }

  function renderCustom() {
    root.querySelectorAll("li.custom").forEach(li => li.remove());
    st.x.forEach((x, i) => {
      const ul = root.querySelector(`.b-q[data-q="${x.q}"] ul`);
      const li = d.createElement("li");
      li.className = "custom set";
      li.innerHTML = `<span class="b-name"><span class="b-n">${esc(x.n)} <span class="chip">Yours</span></span><span class="b-ref none"><span>Only on your radar</span></span></span><span class="b-ctl"><span class="seg" role="radiogroup" aria-label="Ring for ${esc(x.n)}">${cfg.r.map((r, k) => `<button type="button" role="radio" data-k="${k}" class="r${k}">${meter(k)}<span>${esc(r)}</span></button>`).join("")}</span><button type="button" class="b-rm" data-rm="${i}" aria-label="Remove ${esc(x.n)}" title="Remove">×</button></span>`;
      li.dataset.x = i;
      check(li, x.r);
      ul.appendChild(li);
    });
  }

  root.addEventListener("click", e => {
    const b = e.target.closest(".seg button, .b-rm");
    if (!b) return;
    const li = b.closest("li");
    if (li.dataset.x !== undefined) {
      const i = +li.dataset.x;
      if (b.dataset.rm !== undefined) st.x.splice(i, 1); else st.x[i].r = +b.dataset.k;
    } else {
      const k = +b.dataset.k;
      if (k < 0) delete st.p[li.dataset.id]; else st.p[li.dataset.id] = k;
    }
    render();
  });

  // Radio-group keys: arrows move the choice within a row, Home and End jump to its ends.
  const STEP = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 };
  root.addEventListener("keydown", e => {
    const b = e.target.closest(".seg button");
    if (!b || !(e.key in STEP || e.key === "Home" || e.key === "End")) return;
    e.preventDefault();
    const bs = [...b.closest(".seg").querySelectorAll("button")];
    const i = e.key === "Home" ? 0 : e.key === "End" ? bs.length - 1 : (bs.indexOf(b) + STEP[e.key] + bs.length) % bs.length;
    const li = b.closest("li"), k = bs[i].dataset.k, x = li.dataset.x;
    bs[i].click();
    // Custom rows are rebuilt on render, so find the row again before moving focus.
    const row = li.isConnected ? li : root.querySelector(`li.custom[data-x="${x}"]`);
    if (row) row.querySelector(`.seg button[data-k="${k}"]`).focus();
  });

  const filter = d.getElementById("b-filter");
  filter.addEventListener("input", () => {
    const q = filter.value.trim().toLowerCase();
    rows.forEach(li => { li.hidden = !!q && !li.querySelector(".b-n").textContent.toLowerCase().includes(q); });
  });
  // Reset and Clear replace the whole radar, so each offers an Undo.
  function replace(next, text) {
    const prev = copy(st);
    st = next;
    owner.value = st.owner;
    render();
    say(text, () => { st = prev; owner.value = st.owner; render(); say("Your radar is back."); });
  }
  d.getElementById("b-seed").addEventListener("click", () => replace({ owner: st.owner, p: seed(), x: [] }, `Reset to ${cfg.name}.`));
  d.getElementById("b-clear").addEventListener("click", () => replace({ owner: "", p: {}, x: [] }, "Cleared. Pick a ring for anything you use."));
  d.getElementById("b-add").addEventListener("submit", e => {
    e.preventDefault();
    const name = d.getElementById("b-add-name");
    const n = name.value.trim();
    if (!n) return;
    st.x.push({ n: n.slice(0, 40), q: +d.getElementById("b-add-q").value, r: +d.getElementById("b-add-r").value });
    name.value = "";
    render();
    say(`Added ${n}.`);
  });
  owner.value = st.owner;
  owner.addEventListener("input", () => { st.owner = owner.value.trim(); toml.value = toToml(); save(); });

  const link = () => cfg.base + "#" + encode(st);
  root.addEventListener("click", e => {
    if (!e.target.closest("[data-share]")) return;
    const url = link();
    // Social posts show the site's own card for this link, so the image carries the radar.
    const ok = () => say("Link copied.");
    // No clipboard access: show the link in one selected line to copy by hand.
    const fallback = () => {
      say("Copy this link:", null, 20000);
      msgs.forEach(m => {
        const i = d.createElement("input");
        i.readOnly = true; i.value = url; i.className = "b-url"; i.setAttribute("aria-label", "Share link");
        m.append(" ", i);
        if (m.offsetParent) { i.focus({ preventScroll: true }); i.select(); }
      });
    };
    try { navigator.clipboard.writeText(url).then(ok, fallback); } catch (err) { fallback(); }
  });

  // ---- radar file export ---------------------------------------------------
  function toToml() {
    const who = st.owner || "Your name";
    const today = new Date().toISOString().slice(0, 10);
    const q = s => '"' + s.replace(/\\/g, "\\\\").replace(/"/g, '\\"') + '"';
    let t = `title = ${q(who + "'s radar")}\nowner = ${q(who)}\nkind = "community"\ndescription = "The tools I use, and how much I trust each one."\n\n[[edition]]\ndate = ${today}\ntitle = "First edition"\nnote = "Made with the Product Radar builder."\n`;
    Object.keys(st.p).sort().forEach(id => {
      t += `\n[[blip]]\nentry = "${id}"\nstatus = "confirmed"\nrationale = "Say why it sits in this ring, in one or two sentences."\n[[blip.history]]\ndate = ${today}\nring = "${cfg.ri[st.p[id]]}"\nnote = "Placed in the first edition."\n`;
    });
    if (st.x.length) t += "\n# Not in the catalogue yet. Each needs a data/entries/<id>.toml first:\n" + st.x.map(x => `# - ${x.n} (${cfg.q[x.q]}, ${cfg.r[x.r]})`).join("\n") + "\n";
    return t;
  }

  // ---- image export ------------------------------------------------------------
  d.getElementById("b-png").addEventListener("click", async () => {
    try { await d.fonts.ready; } catch (e) { /* fonts optional */ }
    const W = 1200, H = 1200, c = d.createElement("canvas");
    c.width = W; c.height = H;
    const g = c.getContext("2d");
    const P = { bg: "#0f171d", ink: "#e1e7eb", ink2: "#adb9c2", muted: "#92a1ab", ink3: "#58656f", rule: "#343f47", bands: ["#14362e", "#162c26", "#172320", "#161d1b"], "s-new": "#e0468f", "s-in": "#4a86f0", "s-out": "#e06f22", "s-steady": "#e1e7eb" };
    g.fillStyle = P.bg; g.fillRect(0, 0, W, H);
    const cx = W / 2, cy = 640, s = 1.28;
    for (let k = 3; k >= 0; k--) { g.beginPath(); g.arc(cx, cy, OUTER[k] * s, 0, Math.PI * 2); g.fillStyle = P.bands[k]; g.fill(); g.strokeStyle = P.rule; g.lineWidth = 1.5; g.stroke(); }
    g.beginPath(); g.moveTo(cx - 360 * s, cy); g.lineTo(cx + 360 * s, cy); g.moveTo(cx, cy - 360 * s); g.lineTo(cx, cy + 360 * s); g.stroke();
    const its = items(), pos = layout(its);
    its.forEach(i => {
      const [x, y] = pos.get(i.key);
      if (i.cls === "s-new") { g.beginPath(); g.arc(cx + x * s, cy + y * s, 17, 0, Math.PI * 2); g.strokeStyle = P["s-new"]; g.lineWidth = 3; g.stroke(); }
      g.beginPath(); g.arc(cx + x * s, cy + y * s, 10, 0, Math.PI * 2); g.fillStyle = P[i.cls]; g.fill(); g.strokeStyle = P.bg; g.lineWidth = 3; g.stroke();
    });
    // Every blip named, as on the shared view. Labels use the scope's metric: Martian Mono at
    // 75% width ("condensed"), 0.6em per glyph.
    const ls = labelAll(its, its, pos, F);
    g.strokeStyle = P.muted; g.lineWidth = 1.5;
    ls.filter(l => l.lead).forEach(({ lead: [x0, y0, x1, y1] }) => { g.beginPath(); g.moveTo(cx + x0 * s, cy + y0 * s); g.lineTo(cx + x1 * s, cy + y1 * s); g.stroke(); });
    g.font = `500 ${F * s}px 'Martian Mono', monospace`; g.fontStretch = "condensed"; g.fillStyle = P.ink; g.textBaseline = "alphabetic";
    ls.forEach(l => {
      g.textAlign = l.a === "middle" ? "center" : l.a === "end" ? "right" : "left";
      l.lines.forEach((t, k) => g.fillText(t, cx + l.x * s, cy + (l.y + k * LINE * F) * s));
    });
    g.textAlign = "left"; g.fontStretch = "normal";
    g.font = "800 44px Archivo, sans-serif"; g.fillStyle = P.ink;
    g.fillText((st.owner ? st.owner + "'s" : "My") + " radar", 60, 96);
    g.font = "500 18px 'Martian Mono', monospace"; g.fillStyle = P.ink2;
    g.fillText(cfg.q.join("  ·  "), 60, 136);
    g.fillText("Made with " + cfg.base.replace(/^https:\/\//, ""), 60, H - 40);
    c.toBlob(b => {
      if (!b) { say("Could not create the image."); return; }
      const a = d.createElement("a");
      a.href = URL.createObjectURL(b);
      a.download = (st.owner || "my").toLowerCase().replace(/[^a-z0-9]+/g, "-") + "-radar.png";
      d.body.appendChild(a); a.click(); a.remove();
      setTimeout(() => URL.revokeObjectURL(a.href), 2000);
      say("Image saved.");
    }, "image/png");
  });

  // ---- shared view ---------------------------------------------------------------
  // Someone's radar from a share link, laid out like a community radar page (/r/<id>/): the
  // comparison beside the scope, then every entry side by side with the curated ring.
  let deck = null, sumBox = null, side = null;
  function renderShared(its, cmp) {
    sumBox.innerHTML = `<h2 class="label">Compared with ${esc(cfg.name)}</h2><p class="big"><b>${cmp.same}</b> of ${cmp.both}</p><p>shared entries sit in the same ring. The biggest disagreements come first.</p>` +
      `<ol class="moves">${diffItems(cmp.diffs, "Theirs")}</ol>` + (cmp.left.length ? `<p class="b-left">${leftText(cmp.left)}</p>` : "") +
      `<ul class="legend small"><li><span class="mk s-in" aria-hidden="true"></span> They rate it higher</li><li><span class="mk s-out" aria-hidden="true"></span> They rate it lower</li><li><span class="mk s-new" aria-hidden="true"></span> Only on their radar</li><li><span class="mk s-steady" aria-hidden="true"></span> Same ring</li></ul>` +
      `<p class="b-actions"><button type="button" class="btn" id="b-copy">Edit a copy</button><a class="btn ghost" href="./">Build your own</a></p>`;
    // Side by side: the differences first, across the full width (as on /r/<id>/). The entries
    // in the same ring follow as one short list per ring.
    const name = (id, n) => byId.has(id) ? `<a href="${esc(cfg.entry + id + "/")}">${esc(n)}</a>` : esc(n);
    const gap = i => i.o >= 0 ? Math.abs(i.k - i.o) : -1;
    const differ = its.filter(i => i.k !== i.o).sort((a, b) => gap(b) - gap(a) || a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
    const row = i => {
      const up = i.k < i.o, d = gap(i), by = d === 1 ? "1 ring" : d + " rings";
      const what = i.o < 0 ? '<span class="mk s-new" aria-hidden="true"></span>Only on their radar' : `<span class="mk ${up ? "s-in" : "s-out"}" aria-hidden="true"></span>${up ? "Higher" : "Lower"} by ${by}`;
      return `<tr${i.o >= 0 ? ` class="diff ${up ? "d-in" : "d-out"}"` : ""}><th scope="row">${name(i.key, i.name)}</th><td class="c-q">${esc(cfg.q[i.q])}</td><td>${chip(i.k)}</td><td>${i.o >= 0 ? chip(i.o) : '<span class="quiet">Not placed</span>'}</td><td class="c-d">${what}</td></tr>`;
    };
    const leftRow = c => `<tr><th scope="row">${name(c.id, c.n)}</th><td class="c-q">${esc(cfg.q[c.q])}</td><td><span class="quiet">Not placed</span></td><td>${chip(c.o)}</td><td class="c-d">Left off</td></tr>`;
    const alike = its.filter(i => i.k === i.o);
    const lists = [0, 1, 2, 3].map(k => {
      const ns = alike.filter(i => i.k === k).sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
      return ns.length ? `<div class="ringrow r${k}"><h4>${chip(k)}</h4><ul>${ns.map(i => `<li>${name(i.key, i.name)}</li>`).join("")}</ul></div>` : "";
    }).join("");
    const n = differ.length + cmp.left.length;
    side.innerHTML = `<h2>Side by side</h2><h3 class="cmp-h">Where they differ<span class="count">${n}</span></h3>` +
      (n ? `<div class="tbl"><table class="side"><thead><tr><th scope="col">Entry</th><th scope="col" class="c-q">Quadrant</th><th scope="col">Theirs</th><th scope="col">${esc(cfg.who)}</th><th scope="col">Difference</th></tr></thead><tbody>${differ.map(row).join("")}${cmp.left.map(leftRow).join("")}</tbody></table></div>`
        : `<p class="quiet">Every entry on this radar sits in the same ring as on ${esc(cfg.name)}.</p>`) +
      (alike.length ? `<h3 class="cmp-h">In the same ring<span class="count">${alike.length}</span></h3><div class="same-rings">${lists}</div>` : "");
  }
  if (viewing) {
    d.body.classList.add("viewing");
    const eyebrow = d.querySelector(".page-head .eyebrow span"), h1 = d.querySelector(".page-head h1"), lede = d.querySelector(".page-head .lede");
    eyebrow.textContent = "Shared radar";
    h1.textContent = st.owner ? st.owner + "'s radar" : "A shared radar";
    d.title = h1.textContent + " · Product Radar";
    lede.textContent = `Made with the Product Radar builder. Colour marks where it differs from ${cfg.name}.`;
    deck = d.createElement("section");
    deck.className = "deck wrap";
    deck.setAttribute("aria-label", "Radar");
    deck.innerHTML = '<div class="scope-col"></div><aside class="log compare-sum"></aside>';
    deck.firstChild.appendChild(fig);
    sumBox = deck.lastChild;
    side = d.createElement("section");
    side.className = "wrap compare";
    root.before(deck, side);
    owner.readOnly = true;
    // A copy starts without the other person's name, and never replaces a saved radar unasked.
    sumBox.addEventListener("click", e => {
      if (!e.target.closest("#b-copy")) return;
      const saved = loadSaved();
      // Ask only when the copy would replace a radar of your own that differs from this one.
      const mine = saved && !sameRadar(saved, { p: seed(), x: [] }) && !sameRadar(saved, st);
      if (mine && !confirm(`Replace your saved radar (${Object.keys(saved.p).length + saved.x.length} entries) with a copy of this one?`)) return;
      try { if (saved) localStorage.setItem(PREV, JSON.stringify(saved)); } catch (err) { /* private mode */ }
      viewing = false;
      d.body.classList.remove("viewing");
      root.querySelector(".b-card").insertBefore(fig, stats);
      deck.remove(); side.remove();
      // A copy of your own radar keeps your name; a copy of someone else's starts without one.
      st.owner = saved && sameRadar(saved, st) ? saved.owner : ""; owner.value = st.owner; owner.readOnly = false;
      history.replaceState(null, "", location.pathname);
      eyebrow.textContent = "Your radar";
      h1.textContent = "Build your radar";
      d.title = "Build your radar · Product Radar";
      lede.textContent = `This copy of the shared radar is now yours to edit. Your radar lives in its share link, so you need no account and no server stores it.`;
      render();
      if (mine) say("This copy replaced your saved radar.", () => { st = saved; owner.value = st.owner; render(); say("Your saved radar is back."); });
      else say("This copy is now yours to edit.");
    });
  }
  render();
  // The small scopes measure the quadrant names, drawn in the display font: once the fonts
  // have loaded, lay the labels out again with the names' real size.
  if (d.fonts && d.fonts.status !== "loaded") d.fonts.ready.then(() => { if (!viewing || phone.matches) render(); });

  // From an entry page (build/#e=<id>): open the list at that entry's row.
  const target = hash && !shared && hash.e && root.querySelector(`.b-q li[data-id="${CSS.escape(hash.e)}"]`);
  if (target) {
    target.classList.add("hit");
    target.scrollIntoView({ block: "center" });
    target.querySelector('.seg button[tabindex="0"]').focus({ preventScroll: true });
    setTimeout(() => target.classList.remove("hit"), 2400);
  }
})();
