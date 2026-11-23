// Names a key bind for people: "⌥P" on macOS, "Alt+P" elsewhere.
window.KeyLabel = (() => {
  const mac = navigator.userAgent.includes('Mac')
  const names = {
    Escape: 'Esc', Equal: '=', Minus: '−', Space: 'Space', Enter: 'Enter', Backspace: '⌫',
    ArrowLeft: '←', ArrowRight: '→', ArrowUp: '↑', ArrowDown: '↓',
    BracketLeft: '[', BracketRight: ']', Backquote: '`', Semicolon: ';', Quote: "'",
    Backslash: '\\', Comma: ',', Period: '.', Slash: '/',
    NumpadAdd: 'Num +', NumpadSubtract: 'Num −', NumpadMultiply: 'Num *', NumpadDivide: 'Num /',
    NumpadEnter: 'Num Enter', NumpadDecimal: 'Num .',
  }
  const key = code => names[code] ?? code.replace(/^Key|^Digit/, '').replace(/^Numpad/, 'Num ')
  function format(b) {
    if (!b) return ''
    const mods = mac
      ? [b.ctrl && '⌃', b.alt && '⌥', b.shift && '⇧', b.meta && '⌘'].filter(Boolean).join('')
      : [b.ctrl && 'Ctrl+', b.alt && 'Alt+', b.shift && 'Shift+', b.meta && 'Win+'].filter(Boolean).join('')
    return mods + key(b.code)
  }
  return { format }
})()
