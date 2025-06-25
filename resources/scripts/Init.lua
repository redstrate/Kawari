BASE_DIR = "resources/scripts/"

dofile(BASE_DIR.."commands/Commands.lua")
dofile(BASE_DIR.."actions/Actions.lua")
dofile(BASE_DIR.."events/Events.lua")
dofile(BASE_DIR.."items/Items.lua")
dofile(BASE_DIR.."Global.lua")

-- Lua error handlers, and other server events like player login
function onBeginLogin(player)
    -- send a welcome message
    player:send_message("Welcome to Kawari!")
end

function onCommandRequiredRankInsufficientError(player)
    player:send_message("You do not have permission to run this command.")
end

function onCommandRequiredRankMissingError(additional_information, player)
    local error_msg = "Your script does not define the required_rank variable. Please define it in your script for it to run."

    player:send_message(string.format("%s\nAdditional information: %s", error_msg, additional_information))
end

function onUnknownCommandError(command_name, player)
    player:send_message(string.format("Unknown command %s", command_name))
end
