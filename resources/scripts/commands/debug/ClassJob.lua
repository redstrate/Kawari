function onCommand(args, player)
    local parts = split(args)
    player:set_classjob(parts[1])
end
