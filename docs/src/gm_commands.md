# GM commands

These are commands already built-in into the FFXIV client. Normally only available to "Game Masters", they are also reimplemented in Kawari.

> [!NOTE]
> We lack the original documentation for these commands, so their names and functionality are left up to our interpretation.

| Usage | Details |
| --- | --- |
| `//gm achievementDump` | Dumps information about the client-side achivement list. |
| `//gm aetheryte <on/off> <id/all>` | Unlock an Aetheryte. |
| `//gm blacklist status` | Unknown purpose, looks like it just prints out your blacklist. |
| `//gm charadir` | Opens Windows Explorer to your character user directory. |
| `//gm chr_info <pc/bnpc/enpc/enpc_lively>` | Returns the internal entity IDs for your player character. |
| `//gm collect <amount>` | Subtracts `amount` gil from the targeted player (yourself only for now). |
| `//gm cutflg <complete/incomplete> <id>` | Toggles the seen status of the specified Cutscene ID. |
| `//gm cutsceneflag <complete/incomplete> <id>` | Identical to `//gm cutflg`. |
| `//gm dc_region` | Useless, but returns the DC for the World you set in the config. |
| `//gm howto <on/off> <id>` | Toggles the read status of an Active Help entry. |
| `//gm exp <amount>` | Adds the specified amount of EXP to the current class/job. |
| `//gm fittingshop_ui displayid` | Lists the listed available items under Latest Trends. |
| `//gm fittingshop_ui displayid set <list of ids>` | Sets which items are available under Latest Trends locally. Corresponds to the DisplayId column in the FittingShopCategoryItem Excel sheet. |
| `//gm fittingshop_ui reset` | Resets the available items under Latest Trends back to the list sent by the server. |
| `//gm gc <company id>` | Sets your currently active grand company. If it isn't unlocked, its rank will be set to 1 automatically. 0 = None, 1 = Maelstrom, 2 = Adders, 3 = Flames. |
| `//gm gcrank <rank>` | Sets your currently active grand company's rank. `rank` must be between 1 and 11 as of Dawntrail. |
| `//gm getpos` | Returns your current position. |
| `//gm gil <amount>` | Adds the specified amount of gil to the player. |
| `//gm hp <amount>` | Sets your current HP to the amount specified. |
| `//gm icon <id>` | Sets your online status to the given ID. |
| `//gm item <id>` | Gives yourself an item. This can only place a single item in the first page of your inventory currently. |
| `//gm immediatelyaction 1` | Removes action cooldowns. There is no way to turn this off without logging out. |
| `//gm kill` | Kills the selected player, but only affects you for now. |
| `//gm lv <level>` | Sets your current level. |
| `//gm mp <amount>` | Sets your current MP to the amount specified. |
| `//gm quest accept <id>` | Adds the quest to your active quest list. |
| `//gm quest cancel <id>` | Removes the quest from your active quest list. |
| `//gm quest incomplete <id>` | Removes the quest from the completed quest list. |
| `//gm quest complete <id/all>` | Adds the quest to the completed quest list. |
| `//gm quest sequence <id> <sequence>` | Sets the sequence for this quest. |
| `//gm quest inspect <id> ` | Print information about this quest. |
| `//gm orchestrion <on/off> <id>` | Unlock an Orchestrion song. |
| `//gm pos <x> <y> <z>` | Teleport to the specified location. |
| `//gm race <id>` | Sets your player's race. |
| `//gm getrest` | Returns the current amount of rested EXP. |
| `//gm sex <id>` | Sets your player's sex (0 is male, 1 is female.) |
| `//gm speed <multiplier>` | Increases your movement speed by `multiplier`. |
| `//gm teri <id>` | Changes to the specified territory. |
| `//gm terri <id>` | Identical to `//gm teri`. |
| `//gm teri_info` | Displays information about the current zone. Currently displays zone id, weather, internal zone name, parent region name, and place/display name. |
| `//gm tribe <id>` | Sets your player's tribe. |
| `//gm weather <id>` | Changes the weather. |
| `//gm world` | Prints the current world name. |
| `//gm wireframe` | Toggle wireframe rendering for the environment. |
