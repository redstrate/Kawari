--- TODO: find a way to hardcode it this way
EVENT_ID = 1245187

function Scene00000(player)
    player:play_scene(EVENT_ID, 00000, 4959237, 1)
end

function Scene00001(player)
    --- todo put player in correct position
    player:play_scene(EVENT_ID, 00001, 4959237, 1)
end

function onEnterTerritory(player)
    Scene00000(player);
end

function onSceneFinished(player, scene)
    if scene == 0 then
        Scene00001(player)
    end
end
