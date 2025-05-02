function onBeginLogin(player)
    -- send a welcome message
    player:send_message("Welcome to Kawari!")
end

-- Actions
registerAction(3, "actions/Sprint.lua")
registerAction(9, "actions/FastBlade.lua")

-- Items
registerAction(6221, "items/Fantasia.lua")

