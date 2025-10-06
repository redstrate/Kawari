required_rank = GM_RANK_DEBUG

function onCommand(args, player)
    player:send_message("this is a test of on-screen only", SERVER_NOTICE_ON_SCREEN)
    --player:send_message("this is an on-screen + chat log test", ON_SCREEN + CHAT_LOG)
    player:send_message("this is a test of the chat log only, which omits a param", SERVER_NOTICE_CHAT_LOG)
end
