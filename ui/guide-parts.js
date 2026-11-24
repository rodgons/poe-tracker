// Draws Leveling guide lines for the Guide panel and the Main window's Guide section.
// Each fragment kind has an icon or a type style as well as a colour, never colour alone.
window.GuideParts = (() => {
  const esc = s => String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]))
  const icons = { kill: '⚔ ', quest: '◆ ', waypoint: '⇲ ', waypoint_get: '+⇲ ', logout: '⏏ ', portal: '◌ ', trial: '⛬ ', ascend: '⛬ ', crafting: '⚒ ' }
  function part(p) {
    if (p.kind === 'text') return esc(p.text)
    // a compass direction in degrees, as exile-leveling writes it: 0 is up (north), 90 right
    if (p.kind === 'dir') return `<span class="g-dir" style="transform:rotate(${Number(p.text) - 90}deg)" role="img" aria-label="${esc(p.text)}°">➜</span>`
    return `<span class="g-${esc(p.kind)}">${icons[p.kind] ?? ''}${esc(p.text)}</span>`
  }
  const line = parts => parts.map(part).join('')
  // the shared look of the fragments, in each page's own tokens
  const css = `
    .g-kill { color: var(--kill); font-weight: 600; }
    .g-enter { color: var(--alloc); font-weight: 600; }
    .g-quest { color: var(--next); font-weight: 600; }
    .g-waypoint, .g-waypoint_get, .g-logout, .g-portal, .g-trial, .g-ascend, .g-crafting, .g-area { font-weight: 600; }
    .g-quest_text, .g-generic, .g-arena { color: var(--item); font-style: italic; }
    .g-dir { display: inline-block; color: var(--alloc); }`
  return { esc, line, css }
})()
