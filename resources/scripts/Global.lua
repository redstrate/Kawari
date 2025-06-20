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

function split(input, separator)
    if separator == nil then
        separator = '%s'
    end

    local t = {}
    for str in string.gmatch(input, '([^'..separator..']+)') do
        table.insert(t, str)
    end

    return t
end

-- Constants
GM_RANK_NORMALUSER = 0
GM_RANK_GAMEMASTER = 1
GM_RANK_EVENTJUNIOR = 3
GM_RANK_EVENTSENIOR = 4
GM_RANK_SUPPORT = 5
GM_RANK_SENIOR = 7
GM_RANK_DEBUG = 90
GM_RANK_MAX = 255 -- Doesn't exist, used for purposes of testing permissions in scripts


-- please keep these ids sorted!

-- Actions
registerAction(3, "actions/Sprint.lua")
registerAction(5, "actions/Teleport.lua")
registerAction(9, "actions/FastBlade.lua")

-- Items
registerAction(6221, "items/Fantasia.lua")

-- Events
registerEvent(721028, "tosort/UnendingJourney.lua")
registerEvent(721044, "tosort/CrystalBell.lua")
registerEvent(131079, "warp/WarpInnLimsaLominsa.lua")
registerEvent(131080, "warp/WarpInnGridania.lua")
registerEvent(131081, "warp/WarpInnUldah.lua")
registerEvent(131082, "common/GenericWarp.lua")
registerEvent(131083, "common/GenericWarp.lua")
registerEvent(131084, "common/GenericWarp.lua")
registerEvent(131092, "common/GenericWarp.lua")
registerEvent(131093, "common/GenericWarp.lua")
registerEvent(131094, "common/GenericWarp.lua")
registerEvent(720916, "custom/000/cmndefinnbed_00020.lua")
registerEvent(1245185, "opening/OpeningLimsaLominsa.lua")
registerEvent(1245186, "opening/OpeningGridania.lua")
registerEvent(1245187, "opening/OpeningUldah.lua")

-- TODO: Generic warps might be decided through ArrayEventHandler?

-- Commands
registerCommand("setpos", "commands/debug/SetPos.lua")
registerCommand("classjob", "commands/debug/ClassJob.lua")
registerCommand("setspeed", "commands/debug/SetSpeed.lua")
registerCommand("nudge", "commands/debug/Nudge.lua")
registerCommand("festival", "commands/debug/Festival.lua")
registerCommand("permtest", "commands/debug/PermissionTest.lua")
