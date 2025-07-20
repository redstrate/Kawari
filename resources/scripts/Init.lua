BASE_DIR = "resources/scripts/"

dofile(BASE_DIR.."actions/Actions.lua")
dofile(BASE_DIR.."commands/Commands.lua")
dofile(BASE_DIR.."effects/Effects.lua")
dofile(BASE_DIR.."events/Events.lua")
dofile(BASE_DIR.."items/Items.lua")
dofile(BASE_DIR.."Global.lua")

BED_EVENT_HANDLER = 720916
BED_CUTSCENE_FLAGS = 4165480179 -- TODO: remove this hardcode
BED_SCENE_WAKEUP_ANIM = 00100

-- Lua error handlers, and other server events like player login
function onBeginLogin(player)
    -- send a welcome message
    player:send_message(getLoginMessage())
end

function onFinishZoning(player)
    local zone_id = player.zone.id;

    -- play the wakeup animation
    -- the roost
    -- TODO: check for other inns
    if zone_id == 179 then
        player:start_event(player.id, BED_EVENT_HANDLER, 15, zone_id)
        player:play_scene(player.id, BED_EVENT_HANDLER, BED_SCENE_WAKEUP_ANIM, BED_CUTSCENE_FLAGS, {})
    end
end

function onCommandRequiredRankInsufficientError(player)
    player:send_message("You do not have permission to run this command.")
end

function onCommandRequiredRankMissingError(additional_information, player)
    local error_msg = "Your script does not define the required_rank variable. Please define it in your script for it to run."
    printf(player, "%s\nAdditional information: %s", error_msg, additional_information)
end

function onUnknownCommandError(command_name, player)
    printf(player, "Unknown command %s", command_name)
end
