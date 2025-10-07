# Debug commands

These special debug commands start with `!` and are custom to Kawari.

* `!spawnnpc`: Spawn a NPC for debugging
* `!spawnmonster`: Spawn a monster for debugging
* `!spawnclone`: Spawn a clone of yourself
* `!classjob <id>`: Changes to another class/job
* `!unlock <id>`: Unlock an action, emote, etc. for example: `1` for Return and `4` for Teleport.
* `!equip <name>`: Forcefully equip an item, useful for bypassing class/job and other client restrictions. This will *overwrite* any item in that slot!
* `!nudge <distance> <up/down (optional)>`: Teleport forward, back, up or down `distance` yalms. Specifying up or down will move the player up or down instead of forward or back. Examples: `!nudge 5 up` to move up 5 yalms, `!nudge 5` to move forward 5 yalms, `!nudge -5` to move backward 5 yalms.
* `!festival <id1> <id2> <id3> <id4>`: Sets the festival in the current zone. Multiple festivals can be set together to create interesting effects.
* `!reload`: Reloads `Global.lua` that is normally only loaded once at start-up.
* `!finishevent`: Forcefully finishes the current event, useful if the script has an error and you're stuck talking to something.
* `!item <name>`: Gives you an item matching by name.
* `!inspect`: Prints info about the player.
* `!completeallquests`: Completes every quest in the game, useful for accessing stuff gated behind quest completion.
* `!unlockcontent <id>`: Unlocks the specified instanced content. The ID to use is from the InstanceContent Excel sheet.
* `!replay <path>`: Replays packets, must be in the format generated from cfcap-capture.
* `!condition <name>`: Forcefully sets a condition, see `condition.rs` for what is supported.
* `!clearconditions`: Forcefully clears all conditions set on your character.
* `!acs <category> <param1 (optional)> <param2 (optional)> <param3 (optional)> <param4 (optional)>`: Send an ActorControlSelf to the player.
* `!unlockbuddyequip <id>`: Unlocks the specified BuddyEquip (Companion Barding) ID.
* `!togglemount <id>`: Toggles the unlock status of the specified mount ID.
* `!toggleglassesstyle <id>`: Toggles the unlock status of the specified GlassesStyle ID.
* `!toggleornament <id>`: Toggles the unlock status of the specified ornament ID.
* `!togglechocobotaxistand <id>`: Toggles the unlock status of the specified ChocoboTaxiStand ID.
* `!togglecaughtfish <id>`: Toggles the caught status of the specified fish ID.
* `!togglecaughtspearfish <id>`: Toggles the caught status of the specified fish ID (for Spearfishing).
* `!toggletripletriadcard <id>`: Toggles the unlock status of the specified Triple Triad Card ID.
* `!toggleadventure <id>`: Toggles the unlock status of the specified Adventure (Sightseeing) ID.
* `!toggleminion <id>`: Toggles the unlock status of the specified minion ID.
* `!toggleaethercurrent <id>`: Toggles the unlock status of the specified Aether Current ID.
* `!togglecutsceneseen <id>`: Toggles the seen status of the specified Cutscene ID.
* `!mount <id>`: Allows you to mount in any zone, on the specified mount ID.
