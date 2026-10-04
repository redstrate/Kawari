required_rank = GM_RANK_DEBUG
command_sender = "[reward] "

function onCommand(player, args, name)
    local on_arg = args[1]
    local on = nil

    if on_arg == 0 then
        on = true
    elseif on_arg == 1 then
        on = false
    end

    if on ~= nil then
        local id = args[2]

        if id == 0 then
            player:unlock_all()
            printf(player, "Everything is unlocked, please log in again!")
        else
            -- TODO: support removing unlocks

            player:unlock(id)
            printf(player, "%s unlocked!", id)
        end
    end
end
