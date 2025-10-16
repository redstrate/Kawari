function onTalk(target, player, game_data)
    -- TODO: the client Lua script can't handle the case where *no* quests are unlocked, which is currently the defualt state of the player
    -- but once the initial unlocked quests are unlocked, we can safely run scene 0
    -- (and by can't handle i mean it crashes the game LMAO)
    player:play_scene(target, EVENT_ID, 00001, HIDE_HOTBAR, {0})
end

function onReturn(scene, results, player)
    player:finish_event(EVENT_ID)
end
