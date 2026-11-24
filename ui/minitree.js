// The Mini-tree: draws a panel view as SVG and lets the mouse zoom and move it.
// Shared by the Corner panel and the Main window's Tree section; colours come from the page's CSS variables.
const MiniTree = (() => {
  const R = { small: 46, notable: 78, keystone: 110, mastery: 64, jewel: 70, start: 120 }
  const col = { alloc: 'var(--alloc)', refund: 'var(--refund)', next: 'var(--next)', plan: 'var(--plan)', other: 'var(--other)' }
  const esc = s => String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c])
  // zoom over the whole-spec view (1 = whole spec), centred on the next points
  const MAX_ZOOM = 6
  const clampZoom = z => Math.min(MAX_ZOOM, Math.max(1, z))

  /** A camera: zoom, centre (null follows the Next points), and the last drawn frame. */
  const camera = zoom => ({ zoom: clampZoom(zoom || 1), center: null, frame: null, drag: null })

  // Next-point badge radius in CSS px: badge 1 leads, the rest stay small enough to sit side by side
  const BADGE = { lead: 11, rest: 8.5 }

  /** A Refund's cross: hue-independent, so it never reads as a taken point. */
  const cross = (x, y, s, u) =>
    `<path d="M${x - s} ${y - s}L${x + s} ${y + s}M${x + s} ${y - s}L${x - s} ${y + s}" stroke="var(--refund)" stroke-width="${1.8 * u}" stroke-linecap="round" pointer-events="none"/>`

  /** A Pending refund's counter-clockwise "undo" arrow around the node, open at the top. */
  function undo(x, y, r, u) {
    const a0 = -Math.PI / 3, a1 = a0 - 1.65 * Math.PI
    const P = a => [x + r * Math.cos(a), y + r * Math.sin(a)]
    const [sx, sy] = P(a0), [ex, ey] = P(a1)
    // counter-clockwise tangent at the end, and its normal, for the arrowhead
    const tx = Math.sin(a1), ty = -Math.cos(a1), h = 2.6 * u
    const tip = [ex + tx * h, ey + ty * h], b1 = [ex - ty * h * .8, ey + tx * h * .8], b2 = [ex + ty * h * .8, ey - tx * h * .8]
    return `<path d="M${sx} ${sy}A${r} ${r} 0 1 0 ${ex} ${ey}" fill="none" stroke="var(--ink)" stroke-width="${1.4 * u}" stroke-linecap="round" pointer-events="none"/>`
      + `<path d="M${tip}L${b1}L${b2}Z" fill="var(--ink)" pointer-events="none"/>`
  }

  /** SVG for view `v`, `width` CSS pixels wide with height = width × `ratio`. */
  function draw(v, width, ratio, cam) {
    const by = Object.fromEntries(v.nodes.map(n => [n.id, n]))
    const mine = v.nodes.filter(n => n.s !== 'other'), pad = 160
    // nothing to frame (an empty spec): an empty stage, not a NaN viewBox
    if (!mine.length) return `<svg viewBox="0 0 ${width} ${Math.round(width * ratio)}" xmlns="http://www.w3.org/2000/svg"></svg>`
    let x0 = Math.min(...mine.map(n => n.x)) - pad, y0 = Math.min(...mine.map(n => n.y)) - pad
    let w = Math.max(...mine.map(n => n.x)) - x0 + pad, h = Math.max(...mine.map(n => n.y)) - y0 + pad
    if (h < w * ratio) { y0 -= (w * ratio - h) / 2; h = w * ratio } else { x0 -= (h / ratio - w) / 2; w = h / ratio }
    if (cam.zoom > 1) {
      const focus = v.nodes.filter(n => n.s === 'next').length ? v.nodes.filter(n => n.s === 'next') : mine
      const [cx, cy] = cam.center ?? [focus.reduce((a, n) => a + n.x, 0) / focus.length, focus.reduce((a, n) => a + n.y, 0) / focus.length]
      const zw = w / cam.zoom, zh = h / cam.zoom
      // stay inside the whole-spec frame
      x0 = Math.min(Math.max(cx - zw / 2, x0), x0 + w - zw); y0 = Math.min(Math.max(cy - zh / 2, y0), y0 + h - zh)
      w = zw; h = zh
      if (cam.center) cam.center = [x0 + w / 2, y0 + h / 2]
    }
    cam.frame = { x0, y0, cx: x0 + w / 2, cy: y0 + h / 2, w, h }
    const u = w / width
    const pending = new Set(v.pending || [])
    let g = ''
    for (const [a, b] of v.edges) {
      const A = by[a], B = by[b]
      const taken = n => n.s === 'alloc' || n.s === 'refund'
      const s = taken(A) && taken(B) ? (A.s === 'refund' || B.s === 'refund' ? 'refund' : 'alloc')
        : A.s !== 'other' && B.s !== 'other' && (A.s === 'next' || B.s === 'next') ? 'next'
        : A.s !== 'other' && B.s !== 'other' ? 'plan' : 'other'
      // later paths are dashed, refund paths dotted: neither depends on hue
      const dash = s === 'plan' ? `stroke-dasharray="${4 * u} ${3 * u}"` : s === 'refund' ? `stroke-dasharray="0 ${4 * u}" stroke-linecap="round"` : ''
      g += `<line x1="${A.x}" y1="${A.y}" x2="${B.x}" y2="${B.y}" stroke="${col[s]}" stroke-width="${(s === 'other' ? 1 : s === 'plan' ? 1.4 : 2.4) * u}" ${dash}/>`
    }
    const next = [], names = []
    for (const n of v.nodes) {
      const r = R[n.k] * (n.s === 'other' ? .7 : 1)
      // taken points are solid; Next points and Refunds are hollow, told apart by the badge and the cross
      const fill = n.s === 'alloc' ? 'var(--alloc)' : n.s === 'next' || n.s === 'refund' ? 'var(--scene)' : 'var(--scene2)'
      const attrs = `data-id="${n.id}" fill="${fill}" stroke="${col[n.s]}" stroke-width="${(n.s === 'next' ? 2.4 : 1.4) * u}"`
      g += n.k === 'mastery'
        ? `<rect x="${n.x - r}" y="${n.y - r}" width="${2 * r}" height="${2 * r}" transform="rotate(45 ${n.x} ${n.y})" ${attrs}/>`
        : `<circle cx="${n.x}" cy="${n.y}" r="${r}" ${attrs}/>`
      // the marks keep a minimum on-screen size, so they read even when the whole spec shrinks nodes to dots
      if (n.s === 'refund') g += cross(n.x, n.y, Math.max(r, 4.5 * u) * .75, u)
      if (pending.has(n.id)) g += undo(n.x, n.y, Math.max(r + 4 * u, 8 * u), u)
      // Next points get their badge and name in a last pass, above everything else
      if (n.i) { next.push({ n, r }); continue }
      // other names wait for the Next points' room to be taken, then fill what's left
      const named = n.s === 'refund' || (n.k !== 'small' && n.k !== 'start' && n.k !== 'mastery' && n.s !== 'other')
      if (named) names.push({ n, r, rank: n.s === 'refund' ? 0 : n.k === 'keystone' ? 1 : n.s === 'alloc' ? 2 : 3 })
    }
    g += marks(next, names, u, cam.frame)
    return `<svg style="user-select:none;-webkit-user-select:none" class="minitree${cam.zoom > 1 ? ' zoomed' : ''}${cam.drag ? ' dragging' : ''}" viewBox="${x0} ${y0} ${w} ${h}" role="img" aria-label="Passive tree">${g}</svg>`
  }

  /** The Next points' numbered badges, then every name. Badges are placed in order, so 1 always keeps
   *  its spot on its node; a later badge that would cover an earlier one moves beside its node on a short
   *  leader line. Names go next to their node only where they fit inside the frame without covering a badge
   *  or another name: Next points' first, then Refunds, keystones, taken and later points. A name with no
   *  room is left out (a Next point still shows its number). Badges are drawn last-to-first so 1 is on top. */
  function marks(next, names, u, frame) {
    next.sort((a, b) => a.n.i - b.n.i)
    const discs = [], boxes = []
    const fx0 = frame.x0 + 2 * u, fx1 = frame.x0 + frame.w - 2 * u, fy0 = frame.y0 + 2 * u, fy1 = frame.y0 + frame.h - 2 * u
    const overlaps = (a, b) => !(a.x1 < b.x0 || a.x0 > b.x1 || a.y1 < b.y0 || a.y0 > b.y1)
    const disc = d => ({ x0: d.x - d.r, x1: d.x + d.r, y0: d.y - d.r, y1: d.y + d.r })
    const clearOf = (x, y, r) => discs.every(d => Math.hypot(d.x - x, d.y - y) >= d.r + r + u)
      && boxes.every(b => !overlaps(disc({ x, y, r }), b))
    for (const p of next) {
      const { n, r } = p
      const br = (n.i === 1 ? BADGE.lead : BADGE.rest) * u
      p.b = { x: n.x, y: n.y, r: br }
      if (!clearOf(n.x, n.y, br)) {
        const d = r + br + 6 * u
        search: for (const k of [1, 1.8, 2.6]) for (let j = 0; j < 8; j++) {
          const a = -Math.PI / 4 - j * Math.PI / 4
          const x = n.x + Math.cos(a) * d * k, y = n.y + Math.sin(a) * d * k
          if (clearOf(x, y, br)) { p.b = { x, y, r: br }; break search }
        }
      }
      discs.push(p.b)
    }
    // a name's spots around a disc (a badge, or the node itself): above, below, right, left
    const fs = 10 * u
    function place(name, d, gap) {
      const tw = name.length * fs * .58
      const spots = [
        [d.x, d.y - d.r - gap, 'middle'], [d.x, d.y + d.r + gap + fs * .8, 'middle'],
        [d.x + d.r + gap, d.y + fs * .35, 'start'], [d.x - d.r - gap, d.y + fs * .35, 'end'],
      ]
      for (const [x, y, anchor] of spots) {
        const x0 = anchor === 'middle' ? x - tw / 2 : anchor === 'start' ? x : x - tw
        const box = { x0, x1: x0 + tw, y0: y - fs * .8, y1: y + fs * .2 }
        if (box.x0 < fx0 || box.x1 > fx1 || box.y0 < fy0 || box.y1 > fy1) continue
        if (discs.some(o => overlaps(disc(o), box)) || boxes.some(o => overlaps(o, box))) continue
        boxes.push(box)
        return { x, y, anchor }
      }
      return null
    }
    let leaders = '', badgeSvg = '', nextNames = '', otherNames = ''
    for (const p of next) {
      const name = p.n.n.split(' → ')[0], at = place(name, p.b, 3 * u)
      if (at) nextNames += `<text x="${at.x}" y="${at.y}" font-size="${fs}" text-anchor="${at.anchor}" fill="var(--next)" font-weight="600" stroke="var(--scene)" stroke-width="${3 * u}" stroke-linejoin="round" paint-order="stroke" pointer-events="none">${esc(name)}</text>`
    }
    names.sort((a, b) => a.rank - b.rank)
    for (const { n, r } of names) {
      const name = n.n.split(' → ')[0], at = place(name, { x: n.x, y: n.y, r }, 5 * u)
      if (at) otherNames += `<text x="${at.x}" y="${at.y}" font-size="${fs}" text-anchor="${at.anchor}" fill="${n.s === 'refund' ? 'var(--refund)' : n.s === 'alloc' ? 'var(--alloc)' : 'var(--dim)'}" stroke="var(--scene)" stroke-width="${2.5 * u}" stroke-linejoin="round" paint-order="stroke" pointer-events="none">${esc(name)}</text>`
    }
    for (const { n, r, b } of next.slice().reverse()) {
      const lead = n.i === 1, bfs = (lead ? 12 : 10) * u
      if (b.x !== n.x || b.y !== n.y) {
        const d = Math.hypot(b.x - n.x, b.y - n.y), ux = (b.x - n.x) / d, uy = (b.y - n.y) / d
        leaders += `<line x1="${n.x + ux * r}" y1="${n.y + uy * r}" x2="${b.x - ux * b.r}" y2="${b.y - uy * b.r}" stroke="var(--next)" stroke-width="${1.2 * u}" pointer-events="none"/>`
      }
      badgeSvg += `<circle cx="${b.x}" cy="${b.y}" r="${b.r}" fill="var(--next)"${lead ? ` stroke="var(--ink)" stroke-width="${1.5 * u}"` : ''} data-id="${n.id}"/>`
        + `<text x="${b.x}" y="${b.y + bfs * .36}" font-size="${bfs}" text-anchor="middle" font-family="ui-monospace, Menlo, monospace" font-weight="600" fill="var(--scene)" pointer-events="none">${n.i}</text>`
    }
    return otherNames + leaders + badgeSvg + nextNames
  }

  /** A 14px legend mark, drawn with the same shapes as the tree: 'taken', 'next', 'refund', 'pending', 'later'. */
  function swatch(kind) {
    const body = {
      taken: `<circle cx="7" cy="7" r="4" fill="var(--alloc)"/>`,
      next: `<circle cx="7" cy="7" r="6.5" fill="var(--next)"/><text x="7" y="10" font-size="9" text-anchor="middle" font-family="ui-monospace, Menlo, monospace" font-weight="600" fill="var(--scene)">1</text>`,
      refund: `<circle cx="7" cy="7" r="4" fill="var(--scene)" stroke="var(--refund)" stroke-width="1.2"/>${cross(7, 7, 3.4, 1)}`,
      pending: `<circle cx="7" cy="7" r="2.5" fill="var(--alloc)"/>${undo(7, 7, 5.2, 0.8)}`,
      later: `<line x1="1" y1="7" x2="13" y2="7" stroke="var(--plan)" stroke-width="1.6" stroke-dasharray="3 2"/>`,
    }[kind]
    return `<svg class="swatch" viewBox="0 0 14 14" aria-hidden="true">${body}</svg>`
  }

  /** Mouse on `host` (which holds the SVG): scroll zooms around the pointer, dragging moves
   *  the zoomed view. `changed` redraws; `zoomed` is told the new zoom to remember it. */
  function attach(host, cam, changed, zoomed = () => {}) {
    let queued = false
    const endDrag = () => { if (cam.drag) { cam.drag = null; redraw() } }
    const redraw = () => { if (!queued) { queued = true; requestAnimationFrame(() => { queued = false; changed() }) } }
    // the point of the tree under the pointer, and where the pointer sits in the view (0–1)
    function under(e) {
      const box = host.querySelector('svg.minitree').getBoundingClientRect()
      const fx = (e.clientX - box.left) / box.width, fy = (e.clientY - box.top) / box.height
      return { fx, fy, x: cam.frame.x0 + fx * cam.frame.w, y: cam.frame.y0 + fy * cam.frame.h, box }
    }
    host.addEventListener('wheel', e => {
      if (!cam.frame || !e.target.closest('svg.minitree')) return
      e.preventDefault()
      const p = under(e)
      const before = cam.zoom
      cam.zoom = clampZoom(cam.zoom * Math.exp(-e.deltaY * (e.deltaMode ? 0.05 : 0.002)))
      if (cam.zoom === before) return
      if (cam.zoom === 1) {
        cam.center = null
      } else {
        // keep the point under the pointer still
        const w = cam.frame.w * before / cam.zoom, h = cam.frame.h * before / cam.zoom
        cam.center = [p.x - p.fx * w + w / 2, p.y - p.fy * h + h / 2]
      }
      zoomed(cam.zoom)
      redraw()
    }, { passive: false })
    host.addEventListener('pointerdown', e => {
      if (cam.zoom === 1 || !cam.frame || e.button !== 0 || !e.target.closest('svg.minitree')) return
      e.preventDefault()
      cam.drag = { x: e.clientX, y: e.clientY, cx: cam.frame.cx, cy: cam.frame.cy, scale: cam.frame.w / under(e).box.width }
      host.setPointerCapture(e.pointerId)
      redraw()
    })
    host.addEventListener('pointermove', e => {
      if (!cam.drag) return
      // the button came up somewhere we didn't hear it (the page redrew mid-drag)
      if (!e.buttons) return endDrag()
      cam.center = [cam.drag.cx - (e.clientX - cam.drag.x) * cam.drag.scale, cam.drag.cy - (e.clientY - cam.drag.y) * cam.drag.scale]
      redraw()
    })
    host.addEventListener('pointerup', endDrag)
    host.addEventListener('pointercancel', endDrag)
    host.addEventListener('lostpointercapture', endDrag)
  }

  /** The ⌥ keys: +1 in, -1 out, 0 whole spec; pan moves a third of the view per press. */
  function key(cam, step) {
    cam.zoom = step === 0 ? 1 : clampZoom(cam.zoom * (step > 0 ? 1.5 : 1 / 1.5))
    if (cam.zoom === 1) cam.center = null
  }
  function pan(cam, [dx, dy]) {
    if (cam.zoom === 1 || !cam.frame) return
    cam.center = [cam.frame.cx + dx * cam.frame.w / 3, cam.frame.cy + dy * cam.frame.h / 3]
  }

  return { draw, attach, camera, key, pan, esc, swatch }
})()
