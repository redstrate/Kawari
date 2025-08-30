BASE_DIR = "resources/scripts/"

dofile(BASE_DIR.."actions/Actions.lua")
dofile(BASE_DIR.."commands/Commands.lua")
dofile(BASE_DIR.."effects/Effects.lua")
dofile(BASE_DIR.."eobjs/EObjects.lua")
dofile(BASE_DIR.."events/Events.lua")
dofile(BASE_DIR.."items/Items.lua")
dofile(BASE_DIR.."Global.lua")

-- Lua error handlers, and other server events like player login
function onBeginLogin(player, last_zone_id)
    -- send a welcome message
    player:send_message(getLoginMessage())

    -- decide what to do with the player initially
    if true then
        local starting_town <const> = player.city_state

        if starting_town == 1 then
            -- limsa
            player:change_territory(181)
        elseif starting_town == 2 then
            -- gridania
            player:change_territory(182)
        elseif starting_town == 3 then
            -- ul'dah
            player:change_territory(183)
        end
    else
        player:change_territory(last_zone_id)
    end
end

function onFinishZoning(player)
    local in_inn <const> = player.zone.intended_use == ZONE_INTENDED_USE_INN
    local in_opening <const> = player.zone.intended_use == ZONE_INTENDED_USE_OPENING_AREA

    if in_opening then
        local starting_town <const> = player.city_state

        if starting_town == 1 then
            -- limsa
            player:start_event(player.id, 1245185, 15, 0)
            player:play_scene(player.id, 1245185, 0, 0, {})
        elseif starting_town == 2 then
            -- gridania
            player:start_event(player.id, 1245186, 15, 0)
            player:play_scene(player.id, 1245186, 0, 4959237, {0})
        elseif starting_town == 3 then
            -- ul'dah
            player:start_event(player.id, 1245187, 15, 0)
            player:play_scene(player.id, 1245187, 0, 0, {})
        end
    elseif not in_inn then
        -- Need this first so if a player logs in from a non-inn zone, they won't get the bed scene when they enter. It should only play on login.
        player:set_inn_wakeup(true)
    elseif in_inn and not player.saw_inn_wakeup then
        player:set_inn_wakeup(true)
        -- play the wakeup animation
        player:start_event(player.id, BED_EVENT_HANDLER, 15, player.zone.id)
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
