// The Mini-tree: draws a panel view as SVG and lets the mouse zoom and move it.
// Shared by the Corner panel and the Build window; colours come from the page's CSS variables.
const MiniTree = (() => {
  const R = { small: 46, notable: 78, keystone: 110, mastery: 64, jewel: 70, start: 120 }
  const col = { alloc: 'var(--alloc)', refund: 'var(--refund)', next: 'var(--next)', plan: 'var(--plan)', other: 'var(--other)' }
  const esc = s => String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c])
  // zoom over the whole-spec view (1 = whole spec), centred on the next points
  const MAX_ZOOM = 6
  const clampZoom = z => Math.min(MAX_ZOOM, Math.max(1, z))

  /** A camera: zoom, centre (null follows the Next points), and the last drawn frame. */
  const camera = zoom => ({ zoom: clampZoom(zoom || 1), center: null, frame: null, drag: null })

  /** SVG for view `v`, `width` CSS pixels wide with height = width × `ratio`. */
  function draw(v, width, ratio, cam) {
    const by = Object.fromEntries(v.nodes.map(n => [n.id, n]))
    const mine = v.nodes.filter(n => n.s !== 'other'), pad = 160
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
    let g = ''
    for (const [a, b] of v.edges) {
      const A = by[a], B = by[b]
      const taken = n => n.s === 'alloc' || n.s === 'refund'
      const s = taken(A) && taken(B) ? (A.s === 'refund' || B.s === 'refund' ? 'refund' : 'alloc')
        : A.s !== 'other' && B.s !== 'other' && (A.s === 'next' || B.s === 'next') ? 'next'
        : A.s !== 'other' && B.s !== 'other' ? 'plan' : 'other'
      const dash = s === 'plan' ? `stroke-dasharray="${4 * u} ${3 * u}"` : ''
      g += `<line x1="${A.x}" y1="${A.y}" x2="${B.x}" y2="${B.y}" stroke="${col[s]}" stroke-width="${(s === 'other' ? 1 : s === 'plan' ? 1.4 : 2.4) * u}" ${dash}/>`
    }
    for (const n of v.nodes) {
      const r = R[n.k] * (n.s === 'other' ? .7 : 1)
      const fill = n.s === 'alloc' ? 'var(--alloc)' : n.s === 'refund' ? 'var(--refund)' : n.s === 'next' ? 'var(--scene)' : 'var(--scene2)'
      const attrs = `data-id="${n.id}" fill="${fill}" stroke="${col[n.s]}" stroke-width="${(n.s === 'next' ? 2.4 : 1.4) * u}"`
      g += n.k === 'mastery'
        ? `<rect x="${n.x - r}" y="${n.y - r}" width="${2 * r}" height="${2 * r}" transform="rotate(45 ${n.x} ${n.y})" ${attrs}/>`
        : `<circle cx="${n.x}" cy="${n.y}" r="${r}" ${attrs}/>`
      if (n.i) g += `<circle cx="${n.x}" cy="${n.y}" r="${9.5 * u}" fill="var(--next)" data-id="${n.id}"/><text x="${n.x}" y="${n.y + 3.8 * u}" font-size="${11 * u}" text-anchor="middle" font-family="ui-monospace, Menlo, monospace" font-weight="500" fill="var(--scene)" pointer-events="none">${n.i}</text>`
      const named = n.s === 'next' || n.s === 'refund' || (n.k !== 'small' && n.k !== 'start' && n.k !== 'mastery' && n.s !== 'other')
      if (named) g += `<text x="${n.x}" y="${n.y - r - 5 * u}" font-size="${10 * u}" text-anchor="middle" fill="${n.s === 'next' ? 'var(--next)' : n.s === 'refund' ? 'var(--refund)' : n.s === 'alloc' ? 'var(--alloc)' : 'var(--dim)'}" font-weight="${n.s === 'next' ? 600 : 400}" pointer-events="none">${esc(n.n.split(' → ')[0])}</text>`
    }
    return `<svg class="minitree${cam.zoom > 1 ? ' zoomed' : ''}${cam.drag ? ' dragging' : ''}" viewBox="${x0} ${y0} ${w} ${h}" role="img" aria-label="Passive tree">${g}</svg>`
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

  return { draw, attach, camera, key, pan, esc }
})()
