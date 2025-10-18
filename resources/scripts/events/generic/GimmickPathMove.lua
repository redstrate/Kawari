-- TODO: Do any of the sheets contain any of this info so we don't have to hardcode it?
-- The first value is the ClientPath object on the client-side. The rest of these values are complete unknowns.
TELEPORTER_INFO = {
    [4194305] = { 10114730, 1965, 17743871, 39976, 2158200772 }, -- Teleporter from eastern Aetheryte Plaza to Recreation Zone
    [4194306] = { 10114817, 1966, 17711103, 37571, 2136702993 }, -- Teleporter from Recreation Zone to eastern Aetheryte Plaza
    [4194307] = { 10114878, 1967, 17694720, 32931, 1782416562 }, -- Teleporter from northern Aetheryte Plaza to Government Sector
    [4194308] = { 10114891, 1968, 17727487, 32603, 1896185871 }, -- Teleporter from Government Sector to northern Aetheryte Plaza
    [4194309] = { 10114905, 1969, 11804671, 25409, 1989510302 }, -- Teleporter from Nexus Arcade ground floor to upper balcony
    [4194310] = { 10114944, 1970, 11837439, 25483, 2009628707 }, -- Teleporter from upper balcony to Nexus Arcade ground floor
}

function onEnterTrigger(player)
    player:do_solnine_teleporter(EVENT_ID, table.unpack(TELEPORTER_INFO[EVENT_ID]))
    player:finish_event(EVENT_ID)
end
