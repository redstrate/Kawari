# Tips & Tricks

Kawari can be used to explore and experiment with exploration that would be hard/easily detectable on retail.

## Traveling to cutscene zones

Each "region" in the game that's meant to be traveled to regularly is associated with an ID, which is usually referred to as it's "TerritoryType ID" or "Zone ID". But there are certain locations in-game that don't have an associated ID for one reason or another, usually because it's only meant to be used in a cutscene.

See [the `find-hidden-zones` tool in XIVModTools](https://codeberg.org/redstrate/xivmodtools#find-hidden-zones) for finding said locations, and there's [another small guide to add these zones to your game](https://codeberg.org/redstrate/xivmodtools#add-zone).

If you want anyone on the server to travel to said zone, you can add this modded TerritoryType Excel sheet to the server. See [our guide on Extensibility for more information](extensibility.md).

## Moving around zones faster

With Kawari we have numerous options to aid exploration without a client-side plugin!

* You can switch to a flying camera with the `!spectator` [debug command](debug_commands.md).
* You can use the [`!nudge` debug command](debug_commands.md) to quickly teleport.
* If you know an exact spot or need to travel an extremely long distance, use the `//gm pos <x> <y> <z>` command to teleport there.
* You can quickly mount onto anything with [`!mount` debug command](debug_commands.md).
* You can change your speed with the [`//gm speed <factor>` GM command](gm_commands.md), which applies to the spectator camera and mounts.

## Importing characters from retail

It's possible to import existing characters from the retail server using [Auracite](https://auracite.xiv.zone). You can upload the backup ZIP on the account management page.

## Legacy Mark/Tattoo

This is currently only possible by manually editing the database.

1. Open the database and find the row in the `character_data` table you want to edit.
2. Look at the `chara_make` column, this is where you'll find JSON. You can copy this to a text editor so it's easier to work with.
3. Go to the thirteenth value of the first array, that is the character's facial features. (If you have no facial features selected, this value would be zero.)
4. We need to set the left-most bit. For example: if you don't care about any other facial feature, set this value to 128.

## Replaying old festivals and events

The game regularly rotates various festivals or events throughout the year. Their content is normally unavailable after they finished on retail, however parts of them can still be seen in Kawari. You can set festivals temporarily using the `!festival` [debug command](debug_commands.md), or permanently change the active festivals on [the Admin Panel](https://admin.ffxiv.localhost).

Patch updates may retroactively change or remove older festival content due to the nature of the format.

## Recommended client-side plugins

* cl_showpos for the ease-of-access to position and certain zone information.
* Chronofoil for passively recording packet captures.
