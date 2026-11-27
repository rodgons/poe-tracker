// Draws Leveling guide lines for the Guide panel and the Main window's Guide section.
// Each fragment kind has an icon or a type style as well as a colour, never colour alone.
window.GuideParts = (() => {
  const esc = s => String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]))
  // drawn icons (icons.js), not font glyphs, which some system fonts lack
  const kinds = { kill: 'kill', quest: 'quest', waypoint: 'waypoint', waypoint_get: 'waypoint', logout: 'logout', portal: 'portal', trial: 'trial', ascend: 'trial', crafting: 'crafting' }
  const icon = kind => kinds[kind] && window.Icons ? `${Icons.icon(kinds[kind])}${kind === 'waypoint_get' ? '<b class="g-new">+</b>' : ''} ` : ''
  function part(p) {
    if (p.kind === 'text') return esc(p.text)
    // a compass direction in degrees, as exile-leveling writes it: 0 is up (north), 90 right
    if (p.kind === 'dir') return `<span class="g-dir" style="transform:rotate(${Number(p.text) - 90}deg)" role="img" aria-label="${esc(p.text)}°">➜</span>`
    return `<span class="g-${esc(p.kind)}">${icon(p.kind)}${esc(p.text)}</span>`
  }
  const line = parts => parts.map(part).join('')
  // the Build's gems at a quest hand-in: ★ take as the reward, $ buy from the NPC.
  // The name copies for the game's search; ✓ when the Character already has it socketed.
  const gem = g => `<span class="g-gem ${g.buy ? 'buy' : 'take'}${g.socketed ? ' have' : ''}"><i aria-hidden="true">${g.buy ? '$' : '★'}</i>${g.buy ? 'Buy' : 'Take'} <span class="gcopy" role="button" tabindex="0" data-copy="${esc(g.name)}" title="Click to copy">${esc(g.name)}</span> <small>${esc(g.npc)}</small>${g.socketed ? ' <b title="Already socketed">✓</b>' : ''}</span>`
  const gems = list => list.length ? `<div class="g-gems">${list.map(gem).join('')}</div>` : ''
  // full experience in area levels L ± (3 + L/16): the area's place in that band
  function exp(v) {
    if (v.area_level == null || !v.full_exp) return ''
    const [lo, hi] = v.full_exp, a = v.area_level
    const title = `Level ${v.level}: full experience in areas ${lo}–${hi}`
    if (a < lo) return `<span class="g-exp off" title="${title}">▼ ${lo - a} too low · area ${a}</span>`
    if (a > hi) return `<span class="g-exp off" title="${title}">▲ ${a - hi} too high · area ${a}</span>`
    return `<span class="g-exp" title="${title}">● ${lo} | ${a} | ${hi}</span>`
  }
  // the shared look of the fragments, in each page's own tokens
  const css = `
    .g-kill { color: var(--kill); font-weight: 600; }
    .g-enter { color: var(--alloc); font-weight: 600; }
    .g-quest { color: var(--next); font-weight: 600; }
    .g-waypoint, .g-waypoint_get, .g-logout, .g-portal, .g-trial, .g-ascend, .g-crafting, .g-area { font-weight: 600; }
    .g-quest_text, .g-generic, .g-arena { color: var(--item); font-style: italic; }
    .g-dir { display: inline-block; color: var(--alloc); }
    .g-new { font-weight: 700; margin-left: -1px; }
    .g-gems { padding-left: 18px; margin-top: 2px; display: flex; flex-wrap: wrap; gap: 2px 14px; font-size: .92em; }
    .g-gem i { font-style: normal; display: inline-block; width: 14px; text-align: center; font-weight: 700; }
    .g-gem.take, .g-gem.take i { color: var(--alloc); }
    .g-gem.buy i { color: var(--next); }
    .g-gem small { color: var(--plan); }
    .g-gem.have { opacity: .6; }
    .g-gem b { color: var(--ok); }
    .gcopy { cursor: copy; border-radius: 3px; }
    .gcopy:hover { color: var(--alloc); text-decoration: underline dotted; text-underline-offset: 3px; }
    .gcopy:focus-visible { outline: 2px solid var(--alloc); outline-offset: 1px; }
    .g-copied { margin-left: 6px; font: 600 12px var(--body); color: var(--alloc); }
    .g-exp { font: 600 12.5px var(--body); font-variant-numeric: tabular-nums; white-space: nowrap; color: var(--ok); }
    .g-exp.off { color: var(--refund); }`
  // click (or Enter on) a gem name: copy it through the app, which can write the
  // clipboard when the page has no focus, and say so beside the name for a moment
  function copyOnClick(root) {
    const copy = async el => {
      let ok = true
      try { await window.__TAURI__.core.invoke('copy_text', { text: el.dataset.copy }) } catch { ok = false }
      el.parentElement.querySelector('.g-copied')?.remove()
      el.insertAdjacentHTML('afterend', `<span class="g-copied" role="status">${ok ? 'Copied' : "Couldn't copy"}</span>`)
      const badge = el.nextElementSibling
      setTimeout(() => badge?.remove(), 1400)
    }
    root.addEventListener('click', e => {
      const el = e.target.closest('.gcopy')
      if (el) { e.stopPropagation(); copy(el) }
    }, true)
    root.addEventListener('keydown', e => {
      const el = e.target.closest?.('.gcopy')
      if (el && (e.key === 'Enter' || e.key === ' ')) { e.preventDefault(); e.stopPropagation(); copy(el) }
    }, true)
  }
  return { esc, line, gems, exp, css, copyOnClick }
})()
