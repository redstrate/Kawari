-- A Merchant's Tale

BNPC_YNAZQHA = 12109841 -- This VC's fellow

EVENT_RANGE_FOUNTAIN = 12110117 -- Walking up to the fountain

MAP_EFFECT_LEFT_ROUTE = 11
MAP_EFFECT_CENTER_ROUTE = 13
MAP_EFFECT_RIGHT_ROUTE = 12

TIMELINE_WALL_CLO = 2 -- Corresponds to wall1_clo in bg/ex5/07_mid_m6/shared/for_bg/sgbg_m6d1_a0_gmc01.sgb

-- What route the NPC suggests
local npc_route

function onSetup(director)
    npc_route = math.random(1, 3)

    -- Spawn the cement eater
    director:spawn_bnpc(BNPC_YNAZQHA)
end

function onGimmickRect(director, target)
    if target == EVENT_RANGE_FOUNTAIN then
        director:variant_vote_route(npc_route)
    end
end

function onVariantVote(director, vote)
    if vote == 1 then
        director:map_effect(MAP_EFFECT_LEFT_ROUTE, TIMELINE_WALL_CLO)
    elseif vote == 2 then
        director:map_effect(MAP_EFFECT_CENTER_ROUTE, TIMELINE_WALL_CLO)
    elseif vote == 3 then
        director:map_effect(MAP_EFFECT_RIGHT_ROUTE, TIMELINE_WALL_CLO)
    end
end
