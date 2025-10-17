-- Scene 00000: NPC greeting (usually an animation, sometimes text too?)
-- Scene 00010: Displays shop interface
-- Scene 00255: Seems to be an event termination scene? When used standalone without the proper event sequence, it softlocks.

SCENE_GREETING  = 00000
SCENE_SHOW_SHOP = 00010
SCENE_SHOP_END  = 00255
function onTalk(target, player)
    --[[ Params observed for SCENE_GREETING:
        Gil shops: [0, 1]
        Non- shops: [1, 0]
        MGP shops: [1, 100]
        It's unclear what these mean since shops seem to open and function fine without these.
    ]]
    player:play_scene(target, EVENT_ID, SCENE_GREETING, HIDE_HOTBAR, {0, 1})
end

function onReturn(scene, results, player)
    --[[ Retail uses 221 or 222 u32s as the params to the shop cutscene, representing the buyback list and 1 or 2 additional parameters,
        but it opens fine with a single zero u32 when the buyback list is empty.
        22 u32s are used to represent the ten buyback items. Most of these values are still unknown in meaning, but they likely relate to melds, crafting signature, durability, and more.
        Historically, it seems cutscene 00040 was used instead of 00010 as it is now.
        When the client concludes business with the shop, the scene finishes and returns control to the server. The server will then have the client play scene 255 with no params.
    ]]
    if scene == SCENE_GREETING then
        local buyback_list <const> = player:get_buyback_list(EVENT_ID, true)
        player:play_scene(player.id, EVENT_ID, SCENE_SHOW_SHOP, NO_DEFAULT_CAMERA | HIDE_HOTBAR, buyback_list)
    elseif scene == SCENE_SHOW_SHOP then
        local BUYBACK <const> = 3
        if #results > 0 and results[1] == BUYBACK then -- It shouldn't even be possible to get into a situation where results[1] isn't BUYBACK, but we'll leave it as a guard.
            local item_index <const> = results[2]
            player:do_gilshop_buyback(EVENT_ID, item_index)
            local buyback_list = player:get_buyback_list(EVENT_ID, false)
            buyback_list[1] = BUYBACK
            buyback_list[2] = 100 -- Unknown what this 100 represents: a terminator, perhaps? For sell mode it's 0, while buy and buyback are both 100.
            player:play_scene(player.id, EVENT_ID, SCENE_SHOW_SHOP, NO_DEFAULT_CAMERA | HIDE_HOTBAR, buyback_list)
        elseif #results == 0 then -- The player closed the shop window.
            player:play_scene(player.id, EVENT_ID, SCENE_SHOP_END, NO_DEFAULT_CAMERA | HIDE_HOTBAR, {})
        end
    else
        player:finish_event(EVENT_ID)
    end
end
