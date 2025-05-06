function onBeginLogin(player)
    -- send a welcome message
    player:send_message("Welcome to Kawari!")
end

-- Actions
registerAction(3, "actions/Sprint.lua")
registerAction(9, "actions/FastBlade.lua")

-- Items
registerAction(6221, "items/Fantasia.lua")

-- Events
registerEvent(1245185, "opening/OpeningLimsaLominsa.lua")
registerEvent(1245186, "opening/OpeningGridania.lua")
registerEvent(1245187, "opening/OpeningUldah.lua")
registerEvent(131078, "warp/WarpInnGridania.lua")
registerEvent(131079, "warp/WarpInnLimsaLominsa.lua")
registerEvent(131082, "tosort/LimsaInnDoor.lua")
registerEvent(720916, "custom/000/cmndefinnbed_00020.lua")
