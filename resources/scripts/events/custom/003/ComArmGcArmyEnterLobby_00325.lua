-- Grand Company's Entrance to the Barracks

-- Scenes
SCENE_PROMPT = 00000 -- Enter the barracks?

function onTalk(target, player)
    player:play_scene(SCENE_PROMPT, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    if scene == SCENE_PROMPT then
        if BASE_ID == EOBJ_1 then
            player:change_territory_pop_range(TERRITORY_LIM, POP_RANGE_1)
        elseif BASE_ID == EOBJ_2 then
            player:change_territory_pop_range(TERRITORY_GRD, POP_RANGE_2)
        elseif BASE_ID == EOBJ_3 then
            player:change_territory_pop_range(TERRITORY_ULD, POP_RANGE_3)
        else
            printf(player, "Unknown barracks eobj "..BASE_ID)
        end
    end
    player:finish_event()
end
