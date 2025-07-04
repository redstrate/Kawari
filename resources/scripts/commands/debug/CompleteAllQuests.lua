required_rank = GM_RANK_DEBUG
command_sender = "[completeallquests] "

function onCommand(args, player)
    player:complete_all_quests()
    printf(player, "All quests marked as completed!")
end
