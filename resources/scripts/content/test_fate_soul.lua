-- Test script for "If I Only Had a Soul" FATE

BNPC_MOTIVATION = 4309315

function onSetup(fate)
    fate:spawn_bnpc(BNPC_MOTIVATION)
    fate:set_motivation_npc(BNPC_MOTIVATION)
end
