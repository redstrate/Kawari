--- load defines from Opening Excel sheet, which has this and we don't need to hardcode it'
POS_START = 2299848

function Scene00000(player)
player:play_scene(EVENT_ID, 00000, 4959237, 1)
end

function Scene00001(player)
player:play_scene(EVENT_ID, 00001, 4959237, 1)
end

function onEnterTerritory(player, zone)
--- move the player into the starting position
start_pos = zone:get_pop_range(POS_START)
player:set_position(start_pos)

Scene00000(player);
end

function onSceneFinished(player, scene)
if scene == 0 then
    Scene00001(player)
    end
    end
