/* Product Radar: progressive enhancement. The site works without this file. */
(() => {
  "use strict";
  const d = document, de = d.documentElement;
  const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const esc = s => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

  // Theme toggle; the choice persists per browser.
  const tb = d.querySelector(".theme");
  if (tb) tb.addEventListener("click", () => {
    const cur = de.getAttribute("data-theme") || (matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light");
    const next = cur === "dark" ? "light" : "dark";
    de.setAttribute("data-theme", next);
    try { localStorage.setItem("pr-theme", next); } catch (e) { /* storage unavailable */ }
  });

  // Copy buttons: data-copy="textarea id" or data-copy-text="literal".
  d.addEventListener("click", e => {
    const b = e.target.closest("[data-copy],[data-copy-text]");
    if (!b) return;
    const src = b.dataset.copy ? d.getElementById(b.dataset.copy) : null;
    const text = src ? src.value : b.dataset.copyText;
    const label = b.dataset.label || (b.dataset.label = b.textContent);
    const done = ok => { b.textContent = ok ? "Copied" : "Press Ctrl+C"; setTimeout(() => { b.textContent = label; }, 1600); };
    const fallback = () => { if (src) { src.focus(); src.select(); } done(false); };
    try { navigator.clipboard.writeText(text).then(() => done(true), fallback); } catch (err) { fallback(); }
  });

  const root = d.querySelector("[data-radar]");
  const dataEl = d.getElementById("radar-data");
  if (!root || !dataEl) return;
  const data = JSON.parse(dataEl.textContent);
  const fig = root.querySelector(".scope");
  const svg = fig.querySelector("svg.radar");
  const z = svg.querySelector(".z");
  const tip = fig.querySelector(".tip");
  const blipLayer = svg.querySelector(".blips");
  const STATES = ["s-absent", "s-new", "s-in", "s-out", "s-steady", "s-retired"];
  const latest = data.e.length - 1;
  let cur = latest, zoom = -1, lastPointer = "mouse", tipFor = -1;

  const els = new Map();
  svg.querySelectorAll(".blip").forEach(a => els.set(+a.dataset.i, a));
  const sample = els.values().next().value;
  // Blips retired by the latest edition have no element yet; replay creates them on demand.
  const blipEl = i => {
    let a = els.get(i);
    if (a || !sample) return a;
    a = sample.cloneNode(true);
    a.dataset.i = i;
    a.setAttribute("href", sample.getAttribute("href").replace(/e\/[^/]+\//, "e/" + data.b[i].id + "/"));
    a.setAttribute("class", "blip gone q" + data.b[i].q);
    blipLayer.appendChild(a);
    els.set(i, a);
    return a;
  };

  const moveText = (b, e) => {
    const p = b.p[e]; if (!p) return "";
    const prev = e > 0 && b.p[e - 1] ? b.p[e - 1][2] : -1;
    switch (p[3]) {
      case 1: return "New in " + data.r[p[2]];
      case 2: case 3: return "From " + data.r[prev];
      default: return "No change";
    }
  };

  function showTip(i) {
    const b = data.b[i], p = b.p[cur], a = els.get(i);
    if (!p || !a) return hideTip();
    const st = STATES[p[3]];
    // Edition 01 is the baseline: nothing has changed yet, so the tag says nothing about change.
    const change = cur === 0 ? "" : `<span>${p[3] === 4 ? "" : `<i class="mk ${st}"></i>`}${esc(moveText(b, cur))}</span>`;
    tip.innerHTML = `<b>${esc(b.n)}</b><span>${esc(data.r[p[2]])} · ${esc(data.q[b.q])}</span>${change}` +
      (lastPointer === "touch" ? `<a class="open" href="${esc(a.getAttribute("href"))}">Open entry →</a>` : "");
    tip.classList.toggle("touch", lastPointer === "touch");
    tip.hidden = false;
    const fr = fig.getBoundingClientRect(), br = a.querySelector(".dot").getBoundingClientRect();
    const cx = br.left + br.width / 2 - fr.left, cy = br.top + br.height / 2 - fr.top;
    const tw = tip.offsetWidth, th = tip.offsetHeight;
    let x = cx + 16, y = cy - th - 12;
    if (x + tw > fr.width) x = cx - tw - 16;
    if (x < 0) x = Math.max(0, Math.min(fr.width - tw, cx - tw / 2));
    if (y < 0) y = cy + 18;
    tip.style.transform = `translate(${Math.round(x)}px,${Math.round(y)}px)`;
    tipFor = i;
  }
  function hideTip() { tip.hidden = true; tipFor = -1; }

  // Highlight one blip (and its trail and label), dimming the rest.
  function highlight(i) {
    svg.classList.toggle("hl", i >= 0);
    svg.querySelectorAll(".on").forEach(n => n.classList.remove("on"));
    root.querySelectorAll(".mv.on").forEach(n => n.classList.remove("on"));
    if (i < 0) return;
    svg.querySelectorAll(`[data-i="${i}"]`).forEach(n => n.classList.add("on"));
    root.querySelectorAll(`.mv[data-i="${i}"]`).forEach(n => n.classList.add("on"));
  }

  svg.addEventListener("pointerdown", e => { lastPointer = e.pointerType; });
  svg.addEventListener("pointerover", e => {
    const a = e.target.closest(".blip");
    if (!a || e.pointerType === "touch") return;
    lastPointer = e.pointerType;
    highlight(+a.dataset.i); showTip(+a.dataset.i);
  });
  svg.addEventListener("pointerout", e => {
    const a = e.target.closest(".blip");
    if (!a || e.pointerType === "touch" || (e.relatedTarget && a.contains(e.relatedTarget))) return;
    highlight(-1); hideTip();
  });
  // Touch: first tap shows the data block, the link inside it opens the entry.
  svg.addEventListener("click", e => {
    const a = e.target.closest(".blip");
    const q = e.target.closest(".qlab");
    if (q) { setZoom(zoom === +q.dataset.q ? -1 : +q.dataset.q); return; }
    if (!a) { if (lastPointer === "touch") { hideTip(); highlight(-1); } return; }
    if (lastPointer === "touch" && tipFor !== +a.dataset.i) {
      e.preventDefault(); highlight(+a.dataset.i); showTip(+a.dataset.i);
    }
  });

  // Hovering an entry in the edition log points at its blip.
  root.addEventListener("pointerover", e => {
    const mv = e.target.closest(".mv[data-i]");
    if (mv && e.pointerType !== "touch") { highlight(+mv.dataset.i); showTip(+mv.dataset.i); }
  });
  root.addEventListener("pointerout", e => {
    const mv = e.target.closest(".mv[data-i]");
    if (mv && !(e.relatedTarget && mv.contains(e.relatedTarget))) { highlight(-1); hideTip(); }
  });

  // Quadrant zoom: scale the scope 2x around the chosen quadrant.
  const qbtns = [...root.querySelectorAll(".qsel [data-q]")];
  const qds = [...root.querySelectorAll(".qd[data-q]")];
  const trails = [...svg.querySelectorAll(".trail[data-i]")];
  // Zoom frames one quadrant's box (half the 800-unit view box) plus a thin strip across each axis.
  const VB = 400, STRIP = 14, S = 2 * VB / (VB + STRIP), C = (VB - STRIP) / 2;
  function setZoom(q) {
    zoom = q;
    hideTip();
    highlight(-1);
    svg.classList.remove("zoomed", "zq0", "zq1", "zq2", "zq3");
    if (q < 0) { z.style.transform = ""; }
    else {
      const c = data.c[q], sx = c[1] === "l" ? -1 : 1, sy = c[0] === "t" ? -1 : 1;
      z.style.transform = `scale(${S.toFixed(4)}) translate(${-sx * C}px,${-sy * C}px)`;
      svg.classList.add("zoomed", "zq" + q);
    }
    trails.forEach(t => t.classList.toggle("off", q >= 0 && data.b[+t.dataset.i].q !== q));
    qbtns.forEach(b => b.setAttribute("aria-pressed", String(+b.dataset.q === q)));
    qds.forEach(p => { p.hidden = +p.dataset.q !== q; });
  }
  qbtns.forEach(b => b.addEventListener("click", () => setZoom(+b.dataset.q)));
  d.addEventListener("keydown", e => { if (e.key === "Escape") { setZoom(-1); hideTip(); highlight(-1); } });

  // Replay: move every blip to its position at edition e.
  const range = root.querySelector("#edition");
  const out = root.querySelector(".replay output");
  function go(e) {
    cur = e;
    svg.classList.toggle("past", e !== latest);
    data.b.forEach((b, i) => {
      const p = b.p[e];
      const a = p ? blipEl(i) : els.get(i);
      if (!a) return;
      if (!p) { a.classList.add("gone"); return; }
      a.classList.remove("gone", ...STATES);
      a.classList.add(STATES[p[3]]);
      a.style.transform = `translate(${p[0]}px,${p[1]}px)`;
    });
    if (range) { range.value = e; range.setAttribute("aria-valuetext", data.e[e].s); }
    if (out) out.textContent = data.e[e].s;
    hideTip();
  }
  let timer = 0;
  const play = root.querySelector(".play");
  function stop() { clearInterval(timer); timer = 0; root.classList.remove("playing"); if (play) play.setAttribute("aria-label", "Replay every edition"); }
  if (range) range.addEventListener("input", () => { stop(); go(+range.value); });
  if (play) play.addEventListener("click", () => {
    if (timer) { stop(); return; }
    root.classList.add("playing");
    play.setAttribute("aria-label", "Pause replay");
    let e = cur === latest ? 0 : cur + 1;
    go(e);
    timer = setInterval(() => {
      if (++e > latest) { stop(); return; }
      go(e);
    }, reduce ? 1400 : 1100);
  });

  // One sweep of the scope on first view, pinging each blip as the beam passes.
  let swept = false;
  try { swept = sessionStorage.getItem("pr-swept") === "1"; sessionStorage.setItem("pr-swept", "1"); } catch (e) { /* ignore */ }
  if (!reduce && !swept) {
    const dur = 2.4;
    els.forEach((a, i) => {
      const p = data.b[i].p[latest]; if (!p) return;
      const bearing = (Math.atan2(p[1], p[0]) * 180 / Math.PI + 90 + 360) % 360;
      a.style.setProperty("--d", (bearing / 360 * dur).toFixed(2) + "s");
    });
    fig.classList.add("sweeping");
    setTimeout(() => fig.classList.remove("sweeping"), dur * 1000 + 900);
  }
})();
