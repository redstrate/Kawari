# Scripting Generic Events

There are a surprising amount of "generic events" in the game. This includes objects or NPCs like:

* Doors, specifically the ones that prompt you before leaving an area.
* Warp or gatekeeper NPCs that fulfill the same purpose as doors.
* Innkeepers that teleport you to inn rooms.
* Aetherytes and shards.
* NPCs that have a specific, dedicated role (Gemstone traders, Levemete and Menders for example.)
* Gil shops.

All of these share the same underlying client-side Lua script, and thus can also be handled by the same server-side script too.

## Adding generic events

If you come across an unscripted event in-game, the server will tell you something along the lines of:

> Event 12345 tried to start, but it doesn't have a script associated with it!

This event ID is important, so make sure to save it for the next step. You will need to edit `resources/scripts/events/Events.lua` and find the relevant ID array for your type of NPC. The variable names are obvious, e.g. if you have an unscripted Aetheryte that obviously into the `generic_aetherytes` array.

Once that's complete, make sure to reload scripts by typing `!reload` in chat. Try talking or interacting with the event object again, and it should be functional now!
