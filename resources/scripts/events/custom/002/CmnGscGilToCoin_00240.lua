-- MGP conversion NPC

-- Scenes
SCENE_00000 = 00000 -- Show UI

function convertMGPToGil(mgp)
    return mgp * 10
end

function onTalk(target, player)
    player:play_scene(SCENE_00000, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    if results[1] == 1 then
        local mgp_requested = results[2]
        local gil_needed = convertMGPToGil(mgp_requested)

        player:modify_currency(CURRENCY_GIL, -gil_needed)
        player:modify_currency(CURRENCY_MGP, mgp_requested)
    end

    player:finish_event()
end
