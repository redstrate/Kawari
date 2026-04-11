-- A Merchant's Tale

BNPC_YNAZQHA = 12109841 -- This VC's fellow

EVENT_RANGE_FOUNTAIN = 12110117 -- Walking up to the fountain

-- What route the NPC suggests
local npc_route

function onSetup(director)
    npc_route = math.random(0, 2)

    -- Spawn the cement eater
    director:spawn_bnpc(BNPC_YNAZQHA)
end

function onGimmickRect(director, target)
    if target == EVENT_RANGE_FOUNTAIN then
        director:variant_vote_route(npc_route)
    end
end
