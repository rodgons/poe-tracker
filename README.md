# POE Tracker

An overlay for Path of Exile 1 with a checklist that ticks itself. It reads the game's `Client.txt` log, turns lines into events (zone changes, level-ups, quest skill points, passive allocations, NPC dialogue), and checks off campaign steps in a transparent, click-through window drawn over the game.

Status: research done, no code yet.

## Docs

- [`docs/research/overlay-field-guide.md`](docs/research/overlay-field-guide.md): the full research. Covers what GGG's policy allows, how overlay windows work on each OS, the stack choice (Electron + TypeScript), every log event format with sample lines, area ids, checklist engine design, starter code and build order.
- [`docs/research/overlay-field-guide.html`](docs/research/overlay-field-guide.html): the original illustrated page ([artifact](https://claude.ai/artifact/Vt4D1sThYrhCtfjZPwLYhg)).

## Ground rules (from the policy research)

- Read the log, the clipboard and our own window only. Never read game memory, inject code or edit game files.
- Never let a log event fire a keystroke or chat command on its own. A hotkey may send one command per press.
- Tell the user on first run that the app reads `Client.txt`.

## Planned build order

1. Log tailer and parser printing events in a terminal
2. Bare transparent overlay window (click-through and focus sorted out on macOS)
3. Checklist engine (step JSON, look-ahead matching, manual tick, saved state), tested by replaying real log slices
4. Wire the engine to the overlay over IPC; Act 1 ticks itself
5. Game-window tracking and hotkeys
6. Full campaign data from `exile-leveling`, `/passives` resync, passive-tree checklist, clipboard item checks
