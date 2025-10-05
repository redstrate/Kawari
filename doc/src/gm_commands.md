# GM commands

These GM commands are implemented in the FFXIV protocol, but only some of them are implemented.

* `//gm pos <x> <y> <z>`: Teleport to the specified location
* `//gm teri <id>`: Changes to the specified territory
* `//gm weather <id>`: Changes the weather
* `//gm wireframe`: Toggle wireframe rendering for the environment
* `//gm item <id>`: Gives yourself an item. This can only place a single item in the first page of your inventory currently.
* `//gm lv <level>`: Sets your current level
* `//gm aetheryte <on/off> <id>`: Unlock an Aetheryte.
* `//gm speed <multiplier>`: Increases your movement speed by `multiplier`.
* `//gm orchestrion <on/off> <id>`: Unlock an Orchestrion song.
* `//gm exp <amount>`: Adds the specified amount of EXP to the current class/job.
* `//gm teri_info`: Displays information about the current zone. Currently displays zone id, weather, internal zone name, parent region name, and place/display name.
* `//gm gil <amount>`: Adds the specified amount of gil to the player
* `//gm collect <amount>`: Subtracts `amount` gil from the targeted player (yourself only for now).
* `//gm hp <amount>`: Sets your current HP to the amount specified.
* `//gm mp <amount>`: Sets your current MP to the amount specified.
* `//gm getpos`: Returns your current position.
* `//gm dc_region`: Useless, but returns the DC for the World you set in the config.
* `//gm chr_info <pc/bnpc/enpc/enpc_lively>`: Returns the internal entity IDs for your player character.
