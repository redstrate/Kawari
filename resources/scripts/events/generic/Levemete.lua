-- Generic handler for Levemete giver NPCs

-- TODO: actually implement this menu

-- Scenes
SCENE_00000 = 00000 -- The usual Levemete menu
SCENE_00001 = 00001 -- "Quest of great import" quest, which maybe is used in the initial quests?
SCENE_00002 = 00002 -- Some version of "you cannot use the levemete at the moment"

function onTalk(target, player)
    player:play_scene(SCENE_00002, HIDE_HOTBAR, {0})
end

function onYield(scene, results, player)
    player:finish_event()
end
