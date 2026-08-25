-- Test script for "Clever Girls" FATE

BNPC_TEST = 9001

function onSetup(fate)
    fate:spawn_bnpc(BNPC_TEST)
end

function onActorDeath(fate)
    fate:set_progress(100) -- TODO: Temporary
end
