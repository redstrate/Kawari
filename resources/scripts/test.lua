function onBeginLogin(player)
    -- send a welcome message
    player:send_message("Welcome to Kawari!")
end

function doAction(player)
    -- give sprint
    player:give_status_effect(50, 5.0)
end
