--- load defines from Opening Excel sheet, which has this and we don't need to hardcode it'
POS_START = 2299848

function onEnterTerritory(player, zone)
    --- move the player into the starting position
    start_pos = zone:get_pop_range(POS_START)
    player:set_position(start_pos)

    Scene00000(player);
end

function onReturn(scene, results, player)
    if scene == 0 then
        player:play_scene(player.id, EVENT_ID, 1, 4959237, {0})
    end
end
