# Scripting Commands

Scripting commands is super easy, and we support scripting GM and debug commands.

## Debug Commands

This type of command is custom to Kawari and 100% handled by our server. They always start with `!`.

To register a new debug command, enter this into `resources/scripts/commands/Commands.lua`:

```lua
registerCommand("newcommand",          DBG_DIR.."NewCommand.lua")
```

(The rest of this file has plenty of examples.)

## GM Commands

These are pre-defined commands baked into the retail game client, and they always start with `//gm`. The true list and each command's purpose is truly unknown, and some are more obvious than others.

If you have a known GM command, you can register it like so inside of `resources/scripts/commands/Commands.lua`:

```lua
GM_NEW_COMMAND = 69

...

registerGMCommand(GM_NEW_COMMAND,           GM_DIR.."NewCommand.lua")
```

(The rest of this file has plenty of examples.)

## Command Logic

The logic script behind the command is the same for both kinds. You are given an args array and a reference to the `LuaPlayer`. Here is a simple example of changing the player's current territory:

```lua
command_sender = "[teri] "

function onCommand(args, player)
    local id = args[1]

    player:change_territory(id)
    printf(player, "Changing territory to %s.", id)
end
```

Here are some additional or required global variables you can set:
* If you specify `command_sender` printf commands will automatically be prepended with your prefix.
* For GM commands, you must set the `required_rank` global variable for permissions management.

## Documentation

If you plan to upstream your new commands to Kawari, please add them to the relevant documentation pages.
