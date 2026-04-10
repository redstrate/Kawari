required_rank = GM_RANK_DEBUG
command_sender = "[cutsceneflag] "

function onCommand(player, args, name)
    local id = args[1]
    local value = args[2]
    player:toggle_cutscene_seen(tonumber(id), value)
end
