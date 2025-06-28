required_rank = GM_RANK_DEBUG
command_sender = "[teri_info] "

function onCommand(args, player)
    local teri_info = "Territory Info for zone "..player.zone.id..":"
    local current_weather = "Current weather: "..player.zone.weather_id
    local internal_name = "Internal name: "..player.zone.internal_name
    local region_name = "Region name: "..player.zone.region_name
    local place_name = "Place name: "..player.zone.place_name
    printf(player, teri_info.."\n"..current_weather.."\n"..internal_name.."\n"..region_name.."\n"..place_name)
end
