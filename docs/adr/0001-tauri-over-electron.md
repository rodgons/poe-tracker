# Tauri instead of Electron

The field guide (docs/research/overlay-field-guide.md) recommended Electron because Awakened PoE Trade proves an Electron PoE overlay on all three OSes. The owner chose Tauri (Rust + web UI) instead: a much smaller and lighter app that runs alongside the game. We accept the cost of writing per-platform window and input handling ourselves, with no ready-made PoE overlay to copy from. The parser and planning logic stay portable either way.
