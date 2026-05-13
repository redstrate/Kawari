-- Jeffroy at the Phantom Village

-- Scenes
SCENE_MENU = 00001
SCENE_MENU_AGAIN = 00001 -- Unsure when this is used?

function onTalk(target, player)
    player:play_scene(SCENE_MENU, 0, {0})
end

function onReturn(scene, results, player)
    if scene == SCENE_MENU then
        if #results == 2 then
            -- TODO: why does it pass a help menu as the second param?
            local contentIndex = results[1]
            local content
            if contentIndex == 1 then
                content = CONTENT01
            elseif contentIndex == 2 then
                content = CONTENT02
            elseif contentIndex == 3 then
                content = CONTENT03
            else
                printf(player, "Unknown content "..contentIndex)
            end

            player:register_for_content(content)
        end
    end

    player:finish_event()
end
