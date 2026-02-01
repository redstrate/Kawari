-- Khloe Aliapoh in Idyllshire

-- Scene 0: Initial greeting
-- Scene 1: Menu
-- Scene 100: Journal out of date
-- Scene 200: Journal requirements not met
-- Scene 350: Journal requirements met
-- Scene 360: Journal requirements missing(?) or something
-- Scene 300: Reward selection
-- Scene 310: Khloe loves you
-- Scene 400: Journal not available
-- Scene 500: Journal given(?)
-- Scene 510: Journal given 2(?)
-- Scene 600: (I think) journal already complete, but tried to recieve new in the same week
-- Scene 700: Can't recieve duplicate journal, but offer to clear existing one
-- Scene 710: Continuation of 700, if offered to clear existing journal
-- Scene 720: Old rewards available
-- Scene 730: Offer to trade in, similar to scene 100

function onTalk(target, player)
    player:play_scene(target, 0, HIDE_HOTBAR, {})
end

function onYield(scene, results, player)
    if scene == 0 then
        -- Open menu
        player:play_scene(player.id, 1, HIDE_HOTBAR, {})
        return
    elseif scene == 1 then
        if results[1] == 2 then
            -- Journals aren't implemented, so refuse to give one
            player:play_scene(player.id, 400, HIDE_HOTBAR, {})
            return
        end
    end

    player:finish_event()
end
