required_rank = GM_RANK_DEBUG

function onCommand(player, args, name)
    -- Move to Limsa
    player:change_territory(128, { x = 0.0, y = 40.0, z = 0.0 })

    -- Unlock everything
    player:unlock_all()

    -- Complete all quests
    player:finish_quest(65535)

    printf(player, "Please log back out and in again!")
end
