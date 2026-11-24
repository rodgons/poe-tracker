# Glossary

**Build**: A Path of Building build the player wants to follow, imported from a PoB export code or a pastebin link.

**Tree spec**: One of the named passive trees stored inside a Build (e.g. "Level 30", "Final"). The player picks which Tree spec to follow.

**Skill set**: One of the named sets of gems stored inside a Build (e.g. "Level 1-30"), each made of socket groups of gems for one stage of the Build. The Skill set the player follows goes with the chosen Tree spec.
_Avoid_: Loadout, gem setup

**Ascendancy**: The class's own small sub-tree (e.g. Necromancer, Juggernaut) that a Tree spec may include, with its own points earned in the Labyrinth rather than by levelling. Shown apart from the main passive tree, headed by its name.

**Ascendancy points**: The Ascendancy nodes of the chosen Tree spec, counted as taken out of the spec's total (e.g. "4 / 8"), apart from the main tree's count.

**Allocated node**: A passive tree node the player's character currently has.

**Character**: The player's in-game character the app follows, chosen in setup. Every allocation in the Client log is credited to it, except while the log's latest level-up names a different character; then ticking pauses until the Character levels up again or the player re-syncs.

**Seed**: The starting set of Allocated nodes for the Character, taken once at setup or on re-sync. It is imported from the Character's public pathofexile.com profile, or set by hand as "the first N points" of the main tree plus how many Ascendancy points are taken, when the profile is private or the import fails. Live allocations in the Client log change it from then on.

**Next points**: The ordered handful of nodes in the chosen Tree spec that the player should allocate next. Not stored in the Build; derived from the Tree spec and the Allocated nodes. Main-tree only: the Ascendancy's remaining nodes are numbered on their own, apart from the Next points.

**Refund**: An Allocated node that the chosen Tree spec doesn't contain, so the player should take it back. Listed apart from the Next points. An Ascendancy node is a Refund only when the spec has an Ascendancy and the Character's matches it; if the Character's differs, the app shows one notice instead. Cluster jewel and Bloodline nodes are never Refunds; the app leaves them out entirely.

**Pending refund**: An Allocated node the player has clicked to refund in the game but may still Cancel. The game logs the click and nothing on Apply or Cancel, so the node stays Allocated until the next area change, when the app imports the Character from pathofexile.com to settle it (or until a Re-sync, if that import can't run).

**Mini-tree**: A small drawing of the whole Tree spec, showing Allocated nodes, the numbered Next points and the spec's later points, with nearby nodes outside the spec dimmed for context. The spec's Ascendancy, if it has one, is drawn as a separate inset beside it, headed by its name and Ascendancy points. A picture beside the game, not aligned with the game's own tree.

**Corner panel**: The small panel inside the game window (top-right unless the player drags it elsewhere) that holds the Mini-tree, the Tree spec's progress and any Refunds. Toggled by the Panel hotkey while Path of Exile is the frontmost app, and never shown outside the game. The player can zoom and move its Mini-tree with the mouse, but it never takes the keyboard from the game.

**Main window**: The one ordinary app window, away from the game, divided into sections the player moves between: **Tree** (the whole Tree spec large, every point in order, the Refunds and the choice of current Tree spec), **Gems** (the whole Skill set beside the Character's gems, each marked as had, missing or to remove), **Guide** (the whole Leveling guide, where the player can make any Guide step the current one), **Build** (paste the Build, pick the Tree spec, name the Character and set the Seed) and **Settings** (how many Next points, the Client log and the key binds). Opened on first launch and from the tray menu, which can open it at any section. It shows the Character's progress but never changes which nodes are Allocated.
_Avoid_: Setup window, Build window

**Gem panel**: A second small panel inside the game window, apart from the Corner panel, that shows how the Character's socketed gems differ from the Skill set: the gems missing and the gems to remove. Toggled by the Gem panel hotkey, movable, and never shown outside the game.

**Gem panel hotkey**: The key combination, rebindable under Key binds, that toggles the Gem panel. Distinct from the Panel hotkey.

**Panel hotkey**: The key combination (⌥/Alt+P unless the player rebinds it under Key binds in the Main window's Settings section) that toggles the Corner panel. Distinct from the game's own key for opening the passive tree, which the app ignores. While the game is in front the app takes the Panel hotkey from it, because the game ignores modifiers and would otherwise read ⌥/Alt+P as P.

**Client log**: The `Client.txt` file the game appends to as things happen; the source of live allocation events.

**Leveling guide**: The route through the Path of Exile 1 campaign, Acts 1–10, split into Guide steps, telling the player what to do in each zone. It follows the Character on its own: entering the area that ends the current Guide step moves it to the next, and entering the area of a later step skips ahead to that step. It never moves back by itself; any other area (a town, the hideout, an earlier zone) leaves it where it is.
_Avoid_: Act tracker, campaign guide

**Guide step**: One step of the Leveling guide: what to do in the current zone, including which of the Build's gems to take as a quest reward or buy from a quest NPC, ending with the area to enter next. Entering that area completes the step.

**Guide panel**: A third small panel inside the game window, apart from the Corner panel and the Gem panel, that shows the current Guide step. Toggled by the Guide panel hotkey, movable, mouse only, and never shown outside the game.

**Guide panel hotkey**: The key combination, rebindable under Key binds, that toggles the Guide panel. Distinct from the Panel hotkey and the Gem panel hotkey.
