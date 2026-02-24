-- Generic handler for GoldSaucerTalk events

-- Scenes
SCENE_TALK = 0 -- Begin talking

-- Maps some kind of IDs to GoldSaucerTalk IDs
-- TODO: Can this be extracted from the game data somehow?
local gold_saucer_npcs = {
    [2686978] = 161, -- Reymanaud
    [2686980] = 160, -- Wynkyn
}

function onTalk(target, player)
    local scene_id = gold_saucer_npcs[EVENT_ID]
    if scene_id == null then
        player:send_message("Unknown Gold Saucer NPC: "..EVENT_ID)
        player:finish_event()
        return
    end

    player:play_scene(SCENE_TALK, HIDE_HOTBAR, {scene_id})
end

function onReturn(scene, results, player)
    player:finish_event()
end
