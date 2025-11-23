# Debug commands

These special debug commands start with `!` and are custom to Kawari.

| Name | Usage | Details|
| --- | --- | --- |
| `acs` | `!acs <category> <param1 (optional)> <param2 (optional)> <param3 (optional)> <param4 (optional)>` | Send an ActorControlSelf to the player. |
| `completeallquests` | `!completeallquests` | Completes every quest in the game, useful for accessing stuff gated behind quest completion. |
| `condition` | `!condition <name>` | Forcefully sets a condition, see `condition.rs` for what is supported. |
| `classjob` | `!classjob <id>` | Changes to another class/job. |
| `clearconditions` | `!clearconditions` | Forcefully clears all conditions set on your character. |
| `equip` | `!equip <name>` | Forcefully equip an item, useful for bypassing class/job and other client restrictions. This will *overwrite* any item in that slot! |
| `item` | `!item <name>` | Gives you an item matching by name. |
| `inspect` | `!inspect` | Prints info about the player. |
| `festival` | `!festival <id1> <id2> <id3> <id4>` | Sets the festival in the current zone. Multiple festivals can be set together to create interesting effects. |
| `finishevent` | `!finishevent` | Forcefully finishes the current event, useful if the script has an error and you're stuck talking to something. |
| `mount` | `!mount <id>` | Allows you to mount in any zone, on the specified mount ID. |
| `nudge` | `!nudge <distance> <up/down (optional)>` | Teleport forward, back, up or down `distance` yalms. Specifying up or down will move the player up or down instead of forward or back. |
| `reload` | `!reload` | Reloads `Global.lua` that is normally only loaded once at start-up. |
| `unlock` | `!unlock <id>` | Unlock an action, emote, etc. for example: `1` for Return and `4` for Teleport. |
| `unlockbuddyequip` | `!unlockbuddyequip <id>` | Unlocks the specified BuddyEquip (Companion Barding) ID. |
| `unlockcontent` | `!unlockcontent <id>` | Unlocks the specified instanced content. The ID to use is from the InstanceContent Excel sheet. |
| `spawnmonster` | `!spawnmonster <id>` | Spawn a monster for debugging. |
| `spawnclone` | `!spawnclone` | Spawn a clone of yourself. |
| `togglemount` | `!togglemount <id>` | Toggles the unlock status of the specified mount ID. |
| `toggleglassesstyle` | `!toggleglassesstyle <id>` | Toggles the unlock status of the specified GlassesStyle ID. |
| `toggleornament` | `!toggleornament <id>` | Toggles the unlock status of the specified ornament ID. |
| `togglechocobotaxistand` | `!togglechocobotaxistand <id>` | Toggles the unlock status of the specified ChocoboTaxiStand ID. |
| `togglecaughtfish` | `!togglecaughtfish <id>` | Toggles the caught status of the specified fish ID. |
| `togglecaughtspearfish` | `!togglecaughtspearfish <id>` | Toggles the caught status of the specified fish ID (for Spearfishing). |
| `toggletripletriadcard` | `!toggletripletriadcard <id>` | Toggles the unlock status of the specified Triple Triad Card ID. |
| `toggleadventure` | `!toggleadventure <id>` | Toggles the unlock status of the specified Adventure (Sightseeing) ID. |
| `toggleminion` | `!toggleminion <id>` | Toggles the unlock status of the specified minion ID. |
| `toggleaethercurrent` | `!toggleaethercurrent <id>` | Toggles the unlock status of the specified Aether Current ID. |
| `toggleaethercurrentcompflgset` | `!toggleaethercurrentcompflgset <id>` | Toggles the unlock status of the specified AetherCurrentCompFlgSet ID. |
| `togglecutsceneseen` | `!togglecutsceneseen <id>` | Toggles the seen status of the specified Cutscene ID. |
