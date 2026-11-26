// Zone layouts in the Guide panel: pick the start picture that matches the minimap, then
// Directions for the current Guide step (from where the Character came in, past the step's
// objectives, to the exit it names) as one line of compass arrows, with a small drawing of the
// Zone layout that opens a large one on click. Markers differ by shape as well as colour.
window.LayoutParts = (() => {
  const esc = s => String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]))
  const OPEN_KEY = 'poe-tracker.layout-map-open'
  let open = false
  try { open = localStorage.getItem(OPEN_KEY) === '1' } catch {}

  // the node a pick names ("2_1" is child "2_1" of "2"), or null
  function find(nodes, pick) {
    let list = nodes, node = null
    const ids = pick.split('_')
    for (let i = 1; i <= ids.length; i++) {
      node = (list || []).find(n => n.id === ids.slice(0, i).join('_'))
      if (!node) return null
      list = node.children
    }
    return node
  }

  const dist = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1])
  // every ordering of the groups (each group: the alternative spots of one place), each with
  // its best spot, from start to the nearest exit: the shortest straight-leg route
  function shortest(start, groups, exits) {
    let best = null
    const go = (at, left, path, len) => {
      if (best && len >= best.len) return
      if (!left.length) {
        const end = exits.length ? exits.reduce((a, b) => dist(at.at, a.at) <= dist(at.at, b.at) ? a : b) : null
        const total = len + (end ? dist(at.at, end.at) : 0)
        if (!best || total < best.len) best = { len: total, path: end ? [...path, end] : path }
        return
      }
      left.forEach((g, i) => {
        const rest = left.filter((_, j) => j !== i)
        for (const spot of g) go(spot, rest, [...path, spot], len + dist(at.at, spot.at))
      })
    }
    go(start, groups.slice(0, 6), [start], 0)
    return best ? best.path : [start]
  }

  function directions(L, node) {
    const marks = node.marks || []
    const wp = marks.find(m => m.kind === 'waypoint')
    const entrance = marks.find(m => m.kind === 'entrance')
    const start = (L.by_waypoint && wp) || entrance || wp
    if (!start) return null
    const groups = new Map()
    for (const m of marks.filter(m => m.kind === 'objective' && L.objectives.includes(m.name))) {
      // alternative spots of the same place form one group; any other spot is a place of its own
      const key = m.alt ? `${m.name}#${m.order ?? ''}` : `${m.name}@${m.at}`
      groups.set(key, [...(groups.get(key) || []), m])
    }
    if (L.get_waypoint && wp && wp !== start) groups.set('wp', [wp])
    const exits = marks.filter(m => m.kind === 'exit' && m.to === L.exit)
    return { start, path: shortest(start, [...groups.values()], exits), marks }
  }

  // a compass arrow along a leg: 0° is up on the drawing, as the route's own arrows
  function arrow(a, b) {
    const deg = (Math.atan2(b[0] - a[0], -(b[1] - a[1])) * 180 / Math.PI + 360) % 360
    return `<span class="g-dir" style="transform:rotate(${deg - 90}deg)" role="img" aria-label="${Math.round(deg)}°">➜</span>`
  }

  const label = (m, L) => m.kind === 'entrance' ? '<span class="l-start">○</span> start'
    : m.kind === 'waypoint' ? '<span class="l-wp">□</span> Waypoint'
    : m.kind === 'exit' ? `<span class="g-enter">▲ ${esc(L.exit_name || 'exit')}</span>`
    : `<span class="l-obj">◆</span> ${esc(m.name)}`

  // where the player arrives on a start picture (or on a layout offered as a choice)
  const arrival = node => node.start || (node.marks || []).find(m => m.kind === 'entrance')?.at || null

  function bbox(node) {
    const pts = [...node.lines, ...(node.clues || [])].flat()
    const a = arrival(node)
    if (a) pts.push(a)
    if (!pts.length) return [0, 0, 1, 1]
    const xs = pts.map(p => p[0]), ys = pts.map(p => p[1])
    let [x0, y0, x1, y1] = [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)]
    const pad = 0.04, side = Math.max(x1 - x0, y1 - y0) + 2 * pad
    const cx = (x0 + x1) / 2, cy = (y0 + y1) / 2
    return [cx - side / 2, cy - side / 2, side, side]
  }

  // the drawing: outline, clue lines, then the route and markers when given
  function svg(node, size, d, L, crop) {
    const vb = crop ? bbox(node) : [0, 0, 1, 1]
    const u = vb[2] / size  // one screen pixel in drawing units
    const pl = (l, cls) => `<polyline class="${cls}" points="${l.map(p => p.join(',')).join(' ')}"/>`
    let out = node.lines.map(l => pl(l, 'l-wall')).join('') + (node.clues || []).map(l => pl(l, 'l-clue')).join('')
    const a = crop && arrival(node)
    if (a) out += `<circle class="l-m-here" cx="${a[0]}" cy="${a[1]}" r="${4.5 * u}"/>`
    if (d) {
      out += `<polyline class="l-route" points="${d.path.map(m => m.at.join(',')).join(' ')}"/>`
      const on = new Set(d.path)
      for (const m of d.marks) {
        const [x, y] = m.at, r = (on.has(m) ? 5 : 3.5) * u, dim = on.has(m) ? '' : ' l-off'
        if (m.kind === 'entrance') out += `<circle class="l-m-start${dim}" cx="${x}" cy="${y}" r="${r}"/>`
        else if (m.kind === 'waypoint') out += `<rect class="l-m-wp${dim}" x="${x - r}" y="${y - r}" width="${2 * r}" height="${2 * r}"/>`
        else if (m.kind === 'exit') out += `<path class="l-m-exit${dim}" d="M${x},${y - r * 1.2} l${r * 1.1},${r * 2} h${-r * 2.2} z"/>`
        else out += `<rect class="l-m-obj${dim}" x="${x - r * .8}" y="${y - r * .8}" width="${r * 1.6}" height="${r * 1.6}" transform="rotate(45 ${x} ${y})"/>`
      }
    }
    return `<svg class="l-svg" width="${size}" height="${size}" viewBox="${vb.join(' ')}" style="--u:${u}" role="img" aria-label="Zone layout drawing">${out}</svg>`
  }

  function section(L) {
    if (!L) return ''
    const single = L.nodes.length === 1 && !L.nodes[0].children
    let node = L.pick ? find(L.nodes, L.pick) : L.nodes.length === 1 ? L.nodes[0] : null
    // a pick with only one answer needs no click
    while (node && node.children && node.children.length === 1) node = node.children[0]
    const back = L.pick && !single ? '<button class="l-link" data-lpick="">change</button>' : ''
    if (!node || node.children) {
      const choices = node ? node.children : L.nodes
      const thumbs = choices.map((c, i) => `<button class="l-thumb" data-lpick="${esc(c.id)}" aria-label="Layout ${i + 1}" title="This one matches my minimap"><span class="l-n">${i + 1}</span>${svg(c, 110, null, L, true)}</button>`).join('')
      const hint = node ? 'Now, which matches your minimap?' : 'Which matches your minimap where you came in? Look for the green lines.'
      return `<div class="l-pick"><div class="l-hint">${hint} <span class="l-key"><i class="l-here">●</i> you <i class="l-green">━</i> look for this <i class="l-white">━</i> walls</span>${back ? ' ' + back : ''}</div><div class="l-thumbs">${thumbs}</div></div>`
    }
    const d = directions(L, node)
    if (!d) return `<div class="l-row"><span class="l-hint">No start marked on this layout.</span>${back}</div>`
    const legs = [`<span>${label(d.start, L)}</span>`]
    for (let i = 1; i < d.path.length; i++) legs.push(`<span class="l-sep">${arrow(d.path[i - 1].at, d.path[i].at)}</span><span>${label(d.path[i], L)}</span>`)
    const mini = `<button class="l-mini" data-lmap aria-expanded="${open}" title="${open ? 'Hide' : 'Show'} the large drawing">${svg(node, 70, d, L)}</button>`
    return `<div class="l-row"><div class="l-legs">${legs.join('')}</div><span class="l-fill"></span>${mini}${back}</div>`
      + (open ? `<div class="l-big">${svg(node, 320, d, L)}<div class="l-legend"><span><i class="l-start">○</i> you came in</span><span><i class="l-wp">□</i> waypoint</span><span><i class="l-obj">◆</i> objective</span><span><i class="g-enter">▲</i> exit</span><span><i class="g-enter">┅</i> Directions</span></div></div>` : '')
  }

  // clicks: a start picture picks it, "change" goes back to the first pick, the small drawing opens the large one
  function onClick(root, invoke, rerender) {
    root.addEventListener('click', e => {
      const p = e.target.closest('[data-lpick]')
      if (p) { invoke('layout_pick', { pick: p.dataset.lpick || null }); return }
      if (e.target.closest('[data-lmap]')) {
        open = !open
        try { localStorage.setItem(OPEN_KEY, open ? '1' : '0') } catch {}
        rerender()
      }
    })
  }

  const css = `
    .l-row { border-top: 1px solid var(--rule); margin-top: 2px; padding-top: 4px; display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
    .l-hint { font-size: 11px; color: var(--dim); }
    .l-pick { border-top: 1px solid var(--rule); margin-top: 2px; padding-top: 4px; display: flex; flex-direction: column; gap: 5px; }
    .l-key { margin-left: 6px; display: inline-flex; gap: 8px; font-size: 10.5px; color: var(--plan); }
    .l-key i { font-style: normal; }
    .l-here { color: var(--ok); } .l-green { color: #6fe04f; } .l-white { color: var(--faint); }
    .l-thumbs { display: flex; flex-wrap: wrap; gap: 6px; }
    .l-thumb, .l-mini { all: unset; box-sizing: border-box; cursor: pointer; border: 1px solid var(--rule); border-radius: 4px; background: #110f0c; position: relative; display: block; line-height: 0; }
    .l-thumb:hover, .l-mini:hover { border-color: var(--edge); }
    .l-thumb:focus-visible, .l-mini:focus-visible, .l-link:focus-visible { outline: 2px solid var(--alloc); outline-offset: 1px; }
    .l-mini[aria-expanded="true"] { border-color: var(--alloc); }
    .l-n { position: absolute; top: 3px; left: 4px; font: 700 11px var(--mono); line-height: 1; color: var(--alloc); }
    .l-thumb .l-wall { stroke: var(--faint); stroke-width: 1.4; }
    .l-thumb .l-clue { stroke: #6fe04f; stroke-width: 2.6; }
    .l-thumb:hover .l-wall { stroke: var(--plan); }
    .l-m-here { fill: var(--ok); stroke: #11100c; stroke-width: 1.5; vector-effect: non-scaling-stroke; }
    .l-legs { display: flex; flex-wrap: wrap; align-items: center; gap: 2px 6px; font-size: 11.5px; min-width: 0; flex: 1 1 260px; }
    .l-sep { color: var(--plan); }
    .l-fill { flex: 0 0 0; }
    .l-start { color: var(--ok); font-style: normal; } .l-wp { color: var(--ink); font-style: normal; } .l-obj { color: var(--next); font-style: normal; }
    .l-link { all: unset; cursor: pointer; font-size: 10.5px; color: var(--plan); text-decoration: underline dotted; text-underline-offset: 3px; }
    .l-link:hover { color: var(--alloc); }
    .l-big { display: flex; flex-direction: column; gap: 4px; align-items: flex-start; }
    .l-legend { display: flex; flex-wrap: wrap; gap: 2px 10px; font-size: 10.5px; color: var(--dim); }
    .l-legend i { font-style: normal; }
    .l-svg { display: block; background: #110f0c; }
    .l-svg polyline { fill: none; stroke-linejoin: round; stroke-linecap: round; vector-effect: non-scaling-stroke; }
    .l-wall { stroke: var(--plan); stroke-width: 1.2; }
    .l-clue { stroke: var(--ok); stroke-width: 1.6; }
    .l-route { stroke: var(--alloc); stroke-width: 1.8; stroke-dasharray: 5 3; }
    .l-m-start { fill: #11100c; stroke: var(--ok); stroke-width: 1.5; vector-effect: non-scaling-stroke; }
    .l-m-wp { fill: #11100c; stroke: var(--ink); stroke-width: 1.2; vector-effect: non-scaling-stroke; }
    .l-m-exit { fill: var(--alloc); }
    .l-m-obj { fill: var(--next); }
    .l-off { opacity: .35; }`
  return { section, onClick, css }
})()
