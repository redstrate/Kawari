required_rank = GM_RANK_DEBUG
command_sender = "[toggleallcutscene] "

function onCommand(player, args, name)
    player:toggle_cutscene_seen_all()
    printf(player, "Marked all cutscenes as seen!")
end
