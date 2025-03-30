function onBeginLogin(player)
    -- send a welcome message
    player:send_message("Welcome to Kawari!")
end

registerAction(3, "actions/Sprint.lua")
registerAction(9, "actions/FastBlade.lua")
