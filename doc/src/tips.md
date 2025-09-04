# Tips & Tricks

Kawari can be used to explore and experiment with exploration that would be hard/easily detectable on retail.

## Traveling to cutscene zones

Each "region" in the game that's meant to be traveled to regularly is associated with an ID, which is usually referred to as it's "TerritoryType ID" or "Zone ID". But there are certain locations in-game that don't have an associated ID for one reason or another, usually because it's only meant to be used in a cutscene.

See [the `find-hidden-zones` tool in XIVModTools](https://codeberg.org/redstrate/xivmodtools#find-hidden-zones) for finding said locations, and there's [another small guide to add these zones to your game](https://codeberg.org/redstrate/xivmodtools#add-zone). If you want anyone on the server to travel to said zone, you can add this modded TerritoryType Excel sheet to the server. (Guide yet to be written!)

## Moving around zones faster

The game lacks a noclip or flying mode, so getting around zones can appear a bit tough. There's a few solutions to alleviate traveling around some of the bigger zones or through normally inaccessible paths.

* Try changing your speed with the [`//gm speed <factor>` GM command](gm_commands.md).
* You can use the [`!nudge` debug command](debug_commands.md) to quickly teleport.
* If you know an exact spot or need to travel an extremely long distance, use the `//gm pos <x> <y> <z>` command to teleport there.

## Importing characters from retail

It's possible to import existing characters from the retail server using [Auracite](https://auracite.xiv.zone). You can upload the backup ZIP on the account management page.

**NOTE:** This feature is still a work-in-progress, and not all data is imported yet. You also have to use the Dalamud integration, as not all of the required data is provided by the Lodestone alone.

## Legacy Mark/Tattoo

This is currently only possible by manually editing the database.

1. Open the database and find the row in the `character_data` table you want to edit.
2. Look at the `chara_make` column, this is where you'll find JSON. You can copy this to a text editor so it's easier to work with.
3. Go to the thirteenth value of the first array, that is the character's facial features. (If you have no facial features selected, this value would be zero.)
4. We need to set the left-most bit. For example: if you don't care about any other facial feature, set this value to 128.
