BASE_DIR = "resources/scripts/"

dofile(BASE_DIR.."actions/Actions.lua")
dofile(BASE_DIR.."commands/Commands.lua")
dofile(BASE_DIR.."effects/Effects.lua")
dofile(BASE_DIR.."eobjs/EObjects.lua")
dofile(BASE_DIR.."events/Events.lua")
dofile(BASE_DIR.."items/Items.lua")
dofile(BASE_DIR.."Global.lua")

-- As seen on retail
INITIAL_CUTSCENE_FLAGS = NO_DEFAULT_CAMERA | INVIS_ENPC | CONDITION_CUTSCENE | HIDE_UI | HIDE_HOTBAR | SILENT_ENTER_TERRI_ENV | SILENT_ENTER_TERRI_BGM | SILENT_ENTER_TERRI_SE | DISABLE_SKIP | DISABLE_STEALTH

BED_EVENT_HANDLER = 720916
BED_CUTSCENE_FLAGS = 4165480179 -- TODO: remove this hardcode
BED_SCENE_WAKEUP_ANIM = 00100

ZONE_INTENDED_USE_INN = 2
ZONE_INTENDED_USE_OPENING_AREA = 6

-- Lua error handlers, and other server events like player login
function onBeginLogin(player)
    -- send a welcome message
    player:send_message(getLoginMessage())
end

function onFinishZoning(player)
    local in_inn <const> = player.zone.intended_use == ZONE_INTENDED_USE_INN
    local in_opening <const> = player.zone.intended_use == ZONE_INTENDED_USE_OPENING_AREA

    if in_opening then
        local starting_town <const> = player.city_state

        if starting_town == 1 then
            -- limsa
            player:start_event(player.id, 1245185, EVENT_TYPE_ENTER_TERRITORY, 181)
            player:play_scene(player.id, 1245185, 0, INITIAL_CUTSCENE_FLAGS, {0})
        elseif starting_town == 2 then
            -- gridania
            player:start_event(player.id, 1245186, EVENT_TYPE_ENTER_TERRITORY, 183)
            player:play_scene(player.id, 1245186, 0, INITIAL_CUTSCENE_FLAGS, {0})
        elseif starting_town == 3 then
            -- ul'dah
            player:start_event(player.id, 1245187, EVENT_TYPE_ENTER_TERRITORY, 182)
            player:play_scene(player.id, 1245187, 0, INITIAL_CUTSCENE_FLAGS, {0})
        end
    elseif in_inn and not player.saw_inn_wakeup then
        -- play the wakeup animation
        player:start_event(player.id, BED_EVENT_HANDLER, EVENT_TYPE_ENTER_TERRITORY, player.zone.id)
        player:play_scene(player.id, BED_EVENT_HANDLER, BED_SCENE_WAKEUP_ANIM, BED_CUTSCENE_FLAGS, {})
    end

    -- Need this first so if a player logs in from a non-inn zone, they won't get the bed scene when they enter. It should only play on login.
    player:set_inn_wakeup(true)
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
