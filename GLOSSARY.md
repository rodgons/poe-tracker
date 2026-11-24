# Glossary

**Build**: A Path of Building build the player wants to follow, imported from a PoB export code or a pastebin link.

**Tree spec**: One of the named passive trees stored inside a Build (e.g. "Level 30", "Final"). The player picks which Tree spec to follow.

**Skill set**: One of the named sets of gems stored inside a Build (e.g. "Level 1-30"), each made of socket groups of gems for one stage of the Build. The Skill set the player follows goes with the chosen Tree spec.
_Avoid_: Loadout, gem setup

**Allocated node**: A passive tree node the player's character currently has.

**Character**: The player's in-game character the app follows, chosen in setup. Every allocation in the Client log is credited to it, except while the log's latest level-up names a different character; then ticking pauses until the Character levels up again or the player re-syncs.

**Seed**: The starting set of Allocated nodes for the Character, taken once at setup or on re-sync. It is imported from the Character's public pathofexile.com profile, or set by hand as "the first N points" when the profile is private or the import fails. Live allocations in the Client log change it from then on.

**Next points**: The ordered handful of nodes in the chosen Tree spec that the player should allocate next. Not stored in the Build; derived from the Tree spec and the Allocated nodes.

**Refund**: An Allocated node that the chosen Tree spec doesn't contain, so the player should take it back. Listed apart from the Next points. Ascendancy and cluster jewel nodes are never Refunds; the app leaves them out entirely.

**Pending refund**: An Allocated node the player has clicked to refund in the game but may still Cancel. The game logs the click and nothing on Apply or Cancel, so the node stays Allocated until the next area change, when the app imports the Character from pathofexile.com to settle it (or until a Re-sync, if that import can't run).

**Mini-tree**: A small drawing of the whole Tree spec, showing Allocated nodes, the numbered Next points and the spec's later points, with nearby nodes outside the spec dimmed for context. A picture beside the game, not aligned with the game's own tree.

**Corner panel**: The small panel inside the game window (top-right unless the player drags it elsewhere) that holds the Mini-tree, the Tree spec's progress and any Refunds. Toggled by the Panel hotkey while Path of Exile is the frontmost app, and never shown outside the game. The player can zoom and move its Mini-tree with the mouse, but it never takes the keyboard from the game.

**Main window**: The one ordinary app window, away from the game, divided into sections the player moves between: **Tree** (the whole Tree spec large, every point in order, the Refunds and the choice of current Tree spec), **Gems** (the whole Skill set beside the Character's gems, each marked as had, missing or to remove), **Build** (paste the Build, pick the Tree spec, name the Character and set the Seed) and **Settings** (how many Next points, the Client log and the key binds). Opened on first launch and from the tray menu, which can open it at any section. It shows the Character's progress but never changes which nodes are Allocated.
_Avoid_: Setup window, Build window

**Gem panel**: A second small panel inside the game window, apart from the Corner panel, that shows how the Character's socketed gems differ from the Skill set: the gems missing and the gems to remove. Toggled by the Gem panel hotkey, movable, and never shown outside the game.

**Gem panel hotkey**: The key combination, rebindable under Key binds, that toggles the Gem panel. Distinct from the Panel hotkey.

**Panel hotkey**: The key combination (⌥/Alt+P unless the player rebinds it under Key binds in the Main window's Settings section) that toggles the Corner panel. Distinct from the game's own key for opening the passive tree, which the app ignores. While the game is in front the app takes the Panel hotkey from it, because the game ignores modifiers and would otherwise read ⌥/Alt+P as P.

**Client log**: The `Client.txt` file the game appends to as things happen; the source of live allocation events.
