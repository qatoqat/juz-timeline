if (!window.__timelineUi) {
  window.__timelineUi = true;

  const MAX_ZOOM = 40;
  document.addEventListener("wheel", (e) => {
    const scroller = e.target.closest?.(".scroller");
    if (!scroller || Math.abs(e.deltaX) > Math.abs(e.deltaY)) return;
    e.preventDefault();
    const row = scroller.firstElementChild;
    const rect = scroller.getBoundingClientRect();
    const cursor = e.clientX - rect.left;
    const oldWidth = row.offsetWidth;
    const anchor = (scroller.scrollLeft + cursor) / oldWidth;
    const step = e.deltaMode === 1 ? e.deltaY * 16 : e.deltaY;
    const zoom = Math.min(MAX_ZOOM, Math.max(1, (oldWidth / rect.width) * Math.exp(-step * 0.002)));
    row.style.width = `${zoom * 100}%`;
    scroller.scrollLeft = anchor * row.offsetWidth - cursor;
  }, { passive: false });

  const tip = document.createElement("div");
  tip.className = "tip";
  document.body.appendChild(tip);
  let current = null;
  const render = (text) => {
    tip.replaceChildren(...text.split(" · ").map((line) => {
      const row = document.createElement("div");
      const [left, right] = line.split("\t");
      if (right === undefined) {
        row.textContent = line;
      } else {
        row.className = "split";
        const l = document.createElement("span"), r = document.createElement("span");
        l.textContent = left;
        r.className = "parts";
        r.append(...right.split("\u001f").map((part) => {
          const p = document.createElement("span");
          p.textContent = part;
          return p;
        }));
        row.append(l, r);
      }
      return row;
    }));
  };
  const GAP = 8, EDGE = 6;
  const place = (el) => {
    const r = el.getBoundingClientRect();
    const box = el.closest(".scroller")?.getBoundingClientRect();
    const left = Math.max(r.left, box ? box.left : r.left);
    const right = Math.min(r.right, box ? box.right : r.right);
    const w = tip.offsetWidth, h = tip.offsetHeight;
    const x = Math.min(Math.max((left + right) / 2 - w / 2, EDGE), innerWidth - w - EDGE);
    const above = r.top - h - GAP, below = r.bottom + GAP;
    let y = el.classList.contains("bm-handle") ? (above < EDGE ? below : above) : below;
    if (y + h > innerHeight - EDGE) y = above;
    tip.style.transform = `translate(${Math.round(x)}px, ${Math.round(y)}px)`;
  };
  const hide = () => { current = null; tip.classList.remove("on"); };
  let dragging = false;
  document.addEventListener("mouseover", (e) => {
    if (dragging) return;
    const el = e.target.closest?.("[data-tip]");
    if (el === current) return;
    if (!el) return hide();
    current = el;
    render(el.dataset.tip);
    tip.classList.add("on");
    place(el);
  });
  document.addEventListener("scroll", () => current && place(current), true);
  document.addEventListener("wheel", () => current && requestAnimationFrame(() => current && place(current)));
  document.addEventListener("mouseleave", hide);

  const posAt = (e) => {
    let best = null, bestDist = Infinity;
    for (const row of document.querySelectorAll(".row.main")) {
      const r = row.getBoundingClientRect();
      const d = e.clientY < r.top ? r.top - e.clientY : e.clientY > r.bottom ? e.clientY - r.bottom : 0;
      if (d < bestDist) { bestDist = d; best = row; }
    }
    if (!best) return null;
    const r = best.getBoundingClientRect();
    const f = Math.min(1, Math.max(0, (r.right - e.clientX) / r.width));
    const lo = +best.dataset.lo, hi = +best.dataset.hi, mid = best.dataset.mid;
    const p = mid === undefined ? lo + f * (hi - lo) : f <= 0.5 ? lo + f * 2 * (+mid - lo) : +mid + (f - 0.5) * 2 * (hi - +mid);
    return Math.min(p, hi - 0.001);
  };
  let dragIndex = -1;
  const sendPos = (e) => { const p = posAt(e); if (p !== null) dioxus.send([dragIndex, p]); };
  document.addEventListener("pointerdown", (e) => {
    if (e.button !== 0 || !e.target.closest?.(".bmk")) return;
    e.preventDefault();
    const handle = e.target.closest(".bm-handle");
    dragIndex = handle ? +handle.dataset.bm : -1;
    dragging = true;
    document.body.classList.add("dragging");
    sendPos(e);
    const follow = () => {
      if (!dragging) return;
      const handle = document.querySelector(dragIndex >= 0 ? `.bm-handle[data-bm="${dragIndex}"]` : ".bm-handle.active");
      if (handle) {
        current = handle;
        render(handle.dataset.tip);
        tip.classList.add("on");
        place(handle);
      }
      requestAnimationFrame(follow);
    };
    requestAnimationFrame(follow);
  });
  document.addEventListener("pointermove", (e) => dragging && sendPos(e));
  const stop = () => {
    if (!dragging) return;
    dragging = false;
    document.body.classList.remove("dragging");
    hide();
  };
  document.addEventListener("pointerup", stop);
  document.addEventListener("pointercancel", stop);
}
