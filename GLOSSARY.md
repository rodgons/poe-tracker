# Glossary

**Build**: A Path of Building build the player wants to follow, imported from a PoB export code or a pastebin link.

**Tree spec**: One of the named passive trees stored inside a Build (e.g. "Level 30", "Final"). The player picks which Tree spec to follow.

**Allocated node**: A passive tree node the player's character currently has.

**Character**: The player's in-game character the app follows, chosen in setup. Every allocation in the Client log is credited to it, except while the log's latest level-up names a different character; then ticking pauses until the Character levels up again or the player re-syncs.

**Seed**: The starting set of Allocated nodes for the Character, taken once at setup or on re-sync. It is imported from the Character's public pathofexile.com profile, or set by hand as "the first N points" when the profile is private or the import fails. Live allocations in the Client log change it from then on.

**Next points**: The ordered handful of nodes in the chosen Tree spec that the player should allocate next. Not stored in the Build; derived from the Tree spec and the Allocated nodes.

**Refund**: An Allocated node that the chosen Tree spec doesn't contain, so the player should take it back. Listed apart from the Next points. Ascendancy and cluster jewel nodes are never Refunds; the app leaves them out entirely.

**Mini-tree**: A small drawing of the whole Tree spec, showing Allocated nodes, the numbered Next points and the spec's later points, with nearby nodes outside the spec dimmed for context. A picture beside the game, not aligned with the game's own tree.

**Corner panel**: The small window in a screen corner that holds the Mini-tree, the Tree spec's progress and any Refunds. Toggled by the Panel hotkey while Path of Exile is the frontmost app.

**Setup window**: The ordinary app window where the player pastes their Build, picks the Tree spec and names their Character. Opened on first launch and from the tray menu, it is the only window the player clicks in; the Corner panel only shows things.

**Panel hotkey**: The key combination (⌥/Alt+P) that toggles the Corner panel. Distinct from the game's own key for opening the passive tree, which the app ignores.

**Client log**: The `Client.txt` file the game appends to as things happen; the source of live allocation events.
