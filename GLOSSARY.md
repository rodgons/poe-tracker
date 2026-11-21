# Glossary

**Build**: A Path of Building build the player wants to follow, imported from a PoB export code or a pastebin link.

**Tree spec**: One of the named passive trees stored inside a Build (e.g. "Level 30", "Final"). The player picks which Tree spec to follow.

**Allocated node**: A passive tree node the player's character currently has.

**Character**: The player's in-game character the app follows, chosen in setup. Every allocation in the Client log is credited to it.

**Seed**: The starting set of Allocated nodes for the Character, taken once at setup or on re-sync. Live allocations in the Client log change it from then on.

**Next points**: The ordered handful of nodes in the chosen Tree spec that the player should allocate next. Not stored in the Build; derived from the Tree spec and the Allocated nodes.

**Refund**: An Allocated node that the chosen Tree spec doesn't contain, so the player should take it back. Listed apart from the Next points.

**Mini-tree**: A small drawing of the passive tree, zoomed to the area being built into, showing Allocated nodes and the numbered Next points. A picture beside the game, not aligned with the game's own tree.

**Corner panel**: The small window in a screen corner that holds the Mini-tree and the names of the Next points. Toggled by the Panel hotkey while Path of Exile is the frontmost app.

**Panel hotkey**: The key combination (⌥/Alt+P) that toggles the Corner panel. Distinct from the game's own key for opening the passive tree, which the app ignores.

**Client log**: The `Client.txt` file the game appends to as things happen; the source of live allocation events.
