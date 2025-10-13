# GM commands

These are commands already built-in into the FFXIV client. Normally only available to "Game Masters", they are also reimplemented in Kawari.

**Note:** We obviously lack the original documentation for these commands, so the names and functionality are left up to our interpretation.

| Name | Usage | Details |
| --- | --- | --- |
| `aetheryte` | `//gm aetheryte <on/off> <id>` | Unlock an Aetheryte. |
| `blacklist` | `//gm blacklist status` | Unknown purpose, looks like it just prints out your blacklist. |
| `chr_info` | `//gm chr_info <pc/bnpc/enpc/enpc_lively>` | Returns the internal entity IDs for your player character. |
| `collect` | `//gm collect <amount>` | Subtracts `amount` gil from the targeted player (yourself only for now). |
| `dc_region` | `//gm dc_region` | Useless, but returns the DC for the World you set in the config. |
| `exp` | `//gm exp <amount>` | Adds the specified amount of EXP to the current class/job. |
| `getpos` | `//gm getpos` | Returns your current position. |
| `gil` | `//gm gil <amount>` | Adds the specified amount of gil to the player. |
| `hp` | `//gm hp <amount>` | Sets your current HP to the amount specified. |
| `item` | `//gm item <id>` | Gives yourself an item. This can only place a single item in the first page of your inventory currently. |
| `lv` | `//gm lv <level>` | Sets your current level. |
| `mp` | `//gm mp <amount>` | Sets your current MP to the amount specified. |
| `orchestrion` | `//gm orchestrion <on/off> <id>` | Unlock an Orchestrion song. |
| `pos` | `//gm pos <x> <y> <z>` | Teleport to the specified location. |
| `race` | `//gm race <id>` | Sets your player's race. |
| `sex` | `//gm sex <id>` | Sets your player's sex (0 is male, 1 is female.) |
| `speed` | `//gm speed <multiplier>` | Increases your movement speed by `multiplier`. |
| `teri` | `//gm teri <id>` | Changes to the specified territory. |
| `teri_info` | `//gm teri_info` | Displays information about the current zone. Currently displays zone id, weather, internal zone name, parent region name, and place/display name. |
| `tribe` | `//gm tribe <id>` | Sets your player's tribe. |
| `weather` | `//gm weather <id>` | Changes the weather. |
| `wireframe` | `//gm wireframe` | Toggle wireframe rendering for the environment. |
