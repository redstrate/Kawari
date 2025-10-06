required_rank = GM_RANK_DEBUG
command_sender = "[orchestrion] "

function onCommand(args, player)
    local on_arg = args[1]
    local on = nil

    if on_arg == 1 then
        on = true
    elseif on_arg == 2 then
        on = false
    end

    if on ~= nil then
        local id = args[2]

        player:gm_set_orchestrion(on, id)
        printf(player, "Orchestrion(s) %s had their unlocked status changed!", id)
    end
end
