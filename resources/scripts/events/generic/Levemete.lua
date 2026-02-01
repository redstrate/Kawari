-- TODO: actually implement this menu

-- Scene 0: The usual Levemete menu
-- Scene 1: "quest of great import" quest, which maybe is used in the initial quests?
-- Scene 2: some version of "you cannot use the levemete at the moment"

function onTalk(target, player)
    player:play_scene(target, 00002, HIDE_HOTBAR, {0})
end

function onYield(scene, results, player)
    player:finish_event()
end
