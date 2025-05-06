function onBeginLogin(player)
    -- send a welcome message
    player:send_message("Welcome to Kawari!")
end

-- please keep these ids sorted!

-- Actions
registerAction(3, "actions/Sprint.lua")
registerAction(9, "actions/FastBlade.lua")

-- Items
registerAction(6221, "items/Fantasia.lua")

-- Events
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
