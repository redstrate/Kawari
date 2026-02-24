-- Khloe Aliapoh in Idyllshire

-- Scenes
SCENE_00000 = 00000 -- Initial greeting
SCENE_00001 = 00001 -- Menu
SCENE_00100 = 00100 -- Journal out of date
SCENE_00200 = 00200 -- Journal requirements not met
SCENE_00300 = 00300 -- Reward selection
SCENE_00310 = 00310 -- Khloe loves you
SCENE_00350 = 00350 -- Journal requirements met
SCENE_00360 = 00360 -- Journal requirements missing(?) or something
SCENE_00400 = 00400 -- Journal not available
SCENE_00500 = 00500 -- Journal given(?)
SCENE_00510 = 00510 -- Journal given 2(?)
SCENE_00600 = 00600 -- (I think) journal already complete, but tried to recieve new in the same week
SCENE_00700 = 00700 -- Can't recieve duplicate journal, but offer to clear existing one
SCENE_00710 = 00710 -- Continuation of 700, if offered to clear existing journal
SCENE_00720 = 00720 -- Old rewards available
SCENE_00730 = 00730 -- Offer to trade in, similar to scene 100

function onTalk(target, player)
    player:play_scene(SCENE_00000, HIDE_HOTBAR, {})
end

function onReturn(scene, results, player)
    if scene == SCENE_00000 then
        -- Open menu
        player:play_scene(SCENE_00001, HIDE_HOTBAR, {})
        return
    elseif scene == SCENE_00001 then
        if results[1] == 2 then
            -- Journals aren't implemented, so refuse to give one
            player:play_scene(SCENE_00400, HIDE_HOTBAR, {})
            return
        end
    end

    player:finish_event()
end
