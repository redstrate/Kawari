required_rank = GM_RANK_DEBUG

CHAT_LOG = 1
ON_SCREEN = 4

function onCommand(args, player)
    player:send_message("this is a test of on-screen only", ON_SCREEN)
    --player:send_message("this is an on-screen + chat log test", ON_SCREEN + CHAT_LOG)
    player:send_message("this is a test of the chat log only, which omits a param")
end
