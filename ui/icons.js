// Drawn icons for states and Guide fragments, so they look the same on every system: font glyphs
// such as ⛓ and ⛬ are missing from some system fonts. One 1.6px stroke on a 16px grid, in
// currentColor, sized 1em by theme.css (svg.ic). Decorative: the word beside each one carries the meaning.
window.Icons = (() => {
  const svg = body => `<svg class="ic" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`
  const paths = {
    // states
    check: '<path d="M3 8.5 6.4 12 13 4.5"/>',
    plus: '<path d="M8 3v10M3 8h10"/>',
    minus: '<path d="M3 8h10"/>',
    ring: '<circle cx="8" cy="8" r="4.5"/>',
    // a link broken in two: unlinked
    unlink: '<path d="M6.5 4.5 5.3 3.3a2.6 2.6 0 0 0-3.7 3.7l1.9 1.9a2.6 2.6 0 0 0 3 .5"/><path d="M9.5 11.5l1.2 1.2a2.6 2.6 0 0 0 3.7-3.7l-1.9-1.9a2.6 2.6 0 0 0-3-.5"/><path d="M10.5 2.5v1.5M13.5 5.5H12M5.5 13.5V12M2.5 10.5H4"/>',
    // Guide fragments
    kill: '<path d="M2.5 2.5 10 10M10 13.5l3.5-3.5M11.5 11.5 14 14"/><path d="M13.5 2.5 6 10M6 13.5 2.5 10M4.5 11.5 2 14"/>',
    quest: '<path d="M8 1.8 14.2 8 8 14.2 1.8 8z"/>',
    waypoint: '<rect x="2.5" y="2.5" width="11" height="11" rx="1.5"/><path d="M5.5 5.5h5v5M10.5 5.5l-5 5"/>',
    logout: '<path d="M8 2.5v6M4.4 4.6a5 5 0 1 0 7.2 0"/>',
    portal: '<ellipse cx="8" cy="8" rx="4" ry="6"/><ellipse cx="8" cy="8" rx="1.6" ry="3"/>',
    trial: '<path d="M2.5 13.5h11M4 13.5v-7M12 13.5v-7M8 13.5v-7M2.5 6.5 8 2.5l5.5 4z"/>',
    crafting: '<path d="M9.5 2.5 13.5 6.5 11.5 8.5 7.5 4.5z"/><path d="M8.5 7.5 2.5 13.5"/>',
  }
  const icon = name => paths[name] ? svg(paths[name]) : ''
  return { icon }
})()
