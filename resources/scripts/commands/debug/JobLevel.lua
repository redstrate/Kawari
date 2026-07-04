required_rank = GM_RANK_DEBUG

-- !joblevel <classjob_id> <level>
-- Sets the level of a specific ClassJob (by its ClassJob sheet id), regardless of which job is
-- currently active. Unlike the GM SetLevel command (which only touches the active job), this lets
-- you level any job directly — useful for testing job mechanics without grinding.
function onCommand(player, args, name)
    local argc = #args
    if argc ~= 2 then
        printf(player, "Usage: !joblevel <classjob_id> <level>")
        return
    end

    local classjob_id = tonumber(args[1])
    local level = tonumber(args[2])

    if not classjob_id or not level then
        printf(player, "Incorrect arguments given! Usage: !joblevel <classjob_id> <level>")
        return
    end

    if level < 1 or level > 100 then
        printf(player, "Level must be between 1 and 100.")
        return
    end

    player:set_classjob_level(classjob_id, level)
    printf(player, "Set classjob %d to level %d.", classjob_id, level)
end
