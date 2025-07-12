required_rank = GM_RANK_DEBUG
command_sender = "[unlockcontent] "

function onCommand(args, player)
    local id = args[1]
    player:unlock_content(id)
    printf(player, "Content %s unlocked!", id)
end
