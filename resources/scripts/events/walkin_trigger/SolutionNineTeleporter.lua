-- TODO: Do any of the sheets contain any of this info so we don't have to hardcode it?
-- Yes, this table is ugly. Currently these values are complete unknowns.
TELEPORTER_INFO = {
    [4194305] = { 10114730, 1965, 17743871, 2619867137, 2158200772 }, -- Teleporter from eastern Aetheryte Plaza to Recreation Zone
    [4194306] = { 10114817, 1966, 17711103, 2462253057, 2136702993 }, -- Teleporter from Recreation Zone to eastern Aetheryte Plaza
    [4194307] = { 10114878, 1967, 17694720, 2158166017, 1782416562 }, -- Teleporter from northern Aetheryte Plaza to Government Sector
    [4194308] = { 10114891, 1968, 17727487, 2136670209, 1896185871 }, -- Teleporter from Government Sector to northern Aetheryte Plaza
    [4194309] = { 10114905, 1969, 11804671, 1665204225, 1989510302 }, -- Teleporter from Nexus Arcade ground floor to upper balcony
    [4194310] = { 10114944, 1970, 11837439, 1670053889, 2009628707 }, -- Teleporter from upper balcony to Nexus Arcade ground floor
}

EVENT_ARG = {
    [4194305] = 10611851, -- Teleporter from eastern Aetheryte Plaza to Recreation Zone
    [4194306] = 10611861, -- Teleporter from Recreation Zone to eastern Aetheryte Plaza
    [4194307] = 10611862, -- Teleporter from northern Aetheryte Plaza to Government Sector
    [4194308] = 10611864, -- Teleporter from Government Sector to northern Aetheryte Plaza
    [4194309] = 10611868, -- Teleporter from Nexus Arcade ground floor to upper balcony
    [4194310] = 10611881, -- Teleporter from upper balcony to Nexus Arcade ground floor
}

function onEnterTrigger(player)
    player:do_solnine_teleporter(EVENT_ID, table.unpack(TELEPORTER_INFO[EVENT_ID]))
    -- TODO: We should probably take the event arg in Event::new on the rust side, but this works for now.
    player:finish_event(EVENT_ID, EVENT_ARG[EVENT_ID], 1)
end
