-- Please keep these ids sorted in each table!

-- TODO: Generic warps might be decided through ArrayEventHandler?
generic_warps = {
    131077,  -- Ferry Skipper from Old Gridania to East Shroud: Sweetbloom Pier
    131078,  -- Ferry Skipper from East Shroud: Sweetbloom Pier to Old Gridania
    131079, -- Exit Limsa Upper Decks to Mizzenmast Inn room
    131080, -- Exit New Gridania to The Roost Inn room
    131081, -- Exit Ul'dah: Steps of Nald to The Hourglass Inn room
    131082,  -- Exit Mizzenmast Inn to Limsa Upper Decks
    131083,  -- Exit The Roost to New Gridania
    131084,  -- Exit The Hourglass to Ul'dah: Steps of Nald
    131092,  -- Exit from Limsa Bulwark Hall and/or Drowning Wench to Airship Landing
    131093,  -- Exit from Limsa Bullwark Hall and/or Airship Landing to Drowning Wench
    131094,  -- Exit from Limsa Airship Landing and/or Drowning Wench to Bulwark Hall
    131095,  -- Exit from Ul'dah Hustings Strip and/or Ruby Road Exchange to Airship Landing, these three events get reused in several places to ensure thay all connect
    131096,  -- Exit from Ul'dah Airship Landing and/or Ruby Road Exchange to Hustings Strip
    131097,  -- Exit from Ul'dah Airship Landing and/or Husting Strip to Ruby Road Exchange
    --131113,  -- (currently doesn't react, seems to need different handling Ferry Skipper from Upper La Noscea: Memeroon's Trading Post to Upper La Noscea: Jijiroon's Trading Post
    --131114,  -- (currently doesn't react, seems to need different handling Ferry Skipper from Upper La Noscea: Jijiroon's Trading Post to Upper La Noscea: Memeroon's Trading Post
    131126,  -- Gatekeeper from Southern Thanalan: Nald's Reflection to Southern Thanalan: The Minotaur Malm
    --131158, None -- Ferry Skipper from Old Gridania to The Lavender Beds, needs special handling for housing
    --131169,  -- Ferry Skipper from Eastern La Noscea: Costa Del Sol to ELN: Rhotano Privateer (currently broken
    131195,  -- Exit from Fortemps manor to Ishgard: The Pillars
    131204, -- Exit Ishgard: Foundation to Cloud Nine Inn room
    131205,  -- Exit Cloud Nine to Ishgard: Foundation
    131245, -- Exit Kugane to Bokairo Inn room
    131246,  -- Exit Bokairo Inn to Kugane
    131250,  -- Gatekeeper from The Fringes: Castrum Oriens to East Shroud: Amarissaaix's Spire
    131266,  -- Gatekeeper from The House of the Fierce to dead-end cave (unable to dive currently
    131268,  -- Enclave Skiff Captain from The Doman Enclave to Yanxia: The Glittering Basin
    131312,  -- Exit The Pendants Personal Suite to Crystarium
    131313,  -- Exit from The Crown Lift (Lift Operator to Eulmore: The Canopy
    131402,  -- Exit Andron to Old Sharlayan
    131428,  -- Mothercrystal Exit (note: warp doesn't work for some reason?
    131519,  -- Faire Adventurer from Eastern La Noscea: bottom of the Moonfire Festival (2023 tower to the first tier of the tower
    131578,  -- Exit The For'ard Cabins to Tuliyollal    
}

generic_inns = {
    131079, -- Exit Limsa Upper Decks to Mizzenmast Inn room
    131080, -- Exit New Gridania to The Roost Inn room
    131081, -- Exit Ul'dah: Steps of Nald to The Hourglass Inn room
    131204, -- Exit Ishgard: Foundation to Cloud Nine Inn room
    131245, -- Exit Kugane to Bokairo Inn room
}

generic_aetherytes = {
    -- A Realm Reborn Aetherytes
    327683,  -- Bentbranch Meadows Aetheryte
    327684,  -- The Hawthorne Hut Aetheryte
    327685,  -- Quarrymill Aetheryte
    327686,  -- Camp Tranquil Aetheryte
    327687,  -- Fallgourd Float Aetheryte
    327690,  -- Moraby Drydocks Aetheryte
    327691,  -- Costa del Sol Aetheryte
    327692,  -- Wineport Aetheryte
    327693,  -- Swiftperch Aetheryte
    327694,  -- Aleport Aetheryte
    327695,  -- Camp Bronze Lake Aetheryte
    327696,  -- Camp Overlook Aetheryte
    327697,  -- Horizon Aetheryte
    327698,  -- Camp Drybone Aetheryte
    327699,  -- Little Ala Mhigo Aetheryte
    327700,  -- Forgotten Springs Aetheryte
    327701,  -- Camp Bluefog Aetheryte
    327702,  -- Ceruleum Processing Plant Aetheryte
    327703,  -- Camp Dragonhead Aetheryte
    327732,  -- Summerford Farms Aetheryte
    327733,  -- Black Brush Station Aetheryte
    327735,  -- Wolves' Den Pier Aetheryte
    -- registerevent(327???,  -- Ul'dah: Steps of Nald Aetheryte, currently unknown due to the entity not spawning
    -- registerevent(327???,  -- Limsa Lominsa: The Lower Decks Aetheryte, currently unknown due to the entity not spawning
    327742,  -- The Gold Saucer Aetheryte

    -- Heavensward Aetherytes
    327750,  -- Ishgard: Foundation Aetheryte
    327751,  -- Falcon's Nest Aetheryte
    327752,  -- Camp Cloudtop Aetheryte
    327753,  -- Ok' Zundu Aetheryte
    327754,  -- Helix Aetheryte
    327755,  -- Idyllshire Aetheryte
    327756,  -- Tailfeather Aetheryte
    327757,  -- Anyx Trine Aetheryte
    327758,  -- Moghome Aetheryte
    327759,  -- Zenith Aetheryte

    -- Stormblood Aetherytes
    327778,  -- Castrum Oriens Aetheryte
    327779,  -- The Peering Stones Aetheryte
    327780,  -- Ala Gannha Aetheryte
    327781,  -- Ala Ghiri Aetheryte
    327782,  -- Porta Praetoria Aetheryte
    327783,  -- The Ala Mhigan Quarter Aetheryte
    327784,  -- Rhalgr's Reach Aetheryte
    327785,  -- Tamamizu Aetheryte
    327786,  -- Onokoro Aetheryte
    327787,  -- Namai Aetheryte
    327788,  -- The House of the Fierce Aetheryte
    327789,  -- Reunion Aetheryte
    327790,  -- The Dawn Throne Aetheryte
    327791,  -- Kugane Aetheryte
    327807,  -- The Doman Enclave Aetheryte
    327808,  -- Dhoro Iloh Aetheryte

    -- Shadowbringers Aetherytes
    327812,  -- Fort Jobb Aetheryte
    327814,  -- Eulmore Aetheryte
    327816,  -- The Ostal Imperative Aetheryte
    327817,  -- Stilltide Aetheryte
    327818,  -- Wright Aetheryte
    327819,  -- Tomra Aetheryte
    327820,  -- Mord Souq Aetheryte
    327821,  -- Twine Aetheryte
    327822,  -- Slitherbough Aetheryte
    327823,  -- Fanow Aetheryte
    327824,  -- Lydha Lran Aetheryte
    327825,  -- Pla Enni Aetheryte
    327826,  -- Wolekdorf Aetheryte
    327827,  -- The Ondo Cups Aetheryte
    327828,  -- The Macarenses Angle Aetheryte
    327841,  -- The Inn at Journey's Head Aetheryte
    327842,  -- The Doman Enclave: Ferry Docks Aethernet shard
    -- 3278??,  -- The Crystarium Aetheryte, currently unknown due to the entity not spawning

    -- Endwalker Aetherytes
    327846,  -- The Archeion Aetheryte
    327847,  -- Sharlayan Hamlet Aetheryte
    327848,  -- Aporia Aetheryte
    327849,  -- Yedlihmad Aetheryte
    327850,  -- The Great Work Aetheryte
    327851,  -- Palaka's Stand Aetheryte
    327852,  -- Camp Broken Glass Aetheryte
    327853,  -- Tertium Aetheryte
    327854,  -- Sinus Lacrimarum Aetheryte
    327855,  -- Bestways Burrow Aetheryte
    327856,  -- Anagnorisis Aetheryte
    327857,  -- The Twelve Wonders Aetheryte
    327858,  -- Poieten Oikos Aetheryte
    327859,  -- Reah Tahra Aetheryte
    327860,  -- Abode of the Ea Aetheryte
    327861,  -- Base Omicron Aetheryte
    327862,  -- Old Sharlayan Aetheryte
    327863,  -- Radz-at-Han Aetheryte

    -- Dawntrail Aetherytes
    327880,  -- Wachunpelo Aetheryte
    327881,  -- Worlar's Echo Aetheryte
    327882,  -- Ok'hanu Aetheryte
    327883,  -- Many Fires Aetheryte
    327884,  -- Earthenshire Aetheryte
    327885,  -- Iq Br'aax Aetheryte
    327886,  -- Mamook Aetheryte
    327887,  -- Hhusatahwi Aetheryte
    327888,  -- Sheshenewezi Springs Aetheryte
    327889,  -- Mehwahhetsoan Aetheryte
    327890,  -- Yyasulani Station Aetheryte
    327891,  -- The Outskirts Aetheryte
    327892,  -- Electrope Strike Aetheryte
    327893,  -- Leynode Mnemo Aetheryte
    327894,  -- Leynode Pyro Aetheryte
    327895,  -- Leynode Aero Aetheryte
    327896,  -- Tuliyollal Aetheryte
    327897,  -- Solution Nine Aetheryte
    327918,  -- Dock Poga Aetheryte    
}

generic_anetshards = {
    -- A Realm Reborn Aetherytes
    327705,  -- Gridania: Archers' Guild Aethernet shard
    327706,  -- Gridania: Leatherworkers' Guild & Shaded Bower Aethernet shard
    327707,  -- Gridania: Lancers' Guild Aethernet shard
    327708,  -- Gridania: Conjurer' Guild Aethernet shard
    327709,  -- Gridania: Botanists' Guild Aethernet shard
    327710,  -- Gridania: Mih Khetto's Amphitheatre Aethernet shard
    327713,  -- Ul'dah: Adventurers' Guild Aethernet shard
    327714,  -- Ul'dah: Thaumaturges' Guild Aethernet shard
    327715,  -- Ul'dah: Gladiators' Guild Aethernet shard
    327716,  -- Ul'dah: Miners' Guild Aethernet shard
    327717,  -- Ul'dah: Alchemists' Guild Aethernet shard
    327721,  -- Limsa Lominsa: The Aftcastle Aethernet shard
    327722,  -- Limsa Lominsa: Culinarians' Guild Aethernet shard
    327723,  -- Limsa Lominsa: Arcanists' Guild Aethernet shard
    327724,  -- Limsa Lominsa: Fishermen's Guild Aethernet shard
    327727,  -- Ul'dah: Weaver's Guild Aethernet shard
    327728,  -- Limsa Lominsa: Marauders' Guild Aethernet shard
    327729,  -- Limsa Lominsa: Hawker's Alley Aethernet shard
    327730,  -- Ul'dah: Goldsmith's Guild Aethernet shard
    327731,  -- Ul'dah: The Chamber of Rule Aethernet shard

    -- Heavensward Aetherytes
    327760,  -- Ishgard: The Forgotten Knight Aethernet shard
    327761,  -- Ishgard: Skysteel Manufactory Aethernet shard
    327762,  -- Ishgard: The Brume Aethernet shard
    327763,  -- Ishgard: Anathaeum Astrologicum Aethernet shard
    327764,  -- Ishgard: The Jewled Crozier Aethernet shard
    327765,  -- Ishgard: Saint Reymanaud's Cathedral Aethernet shard
    327766,  -- Ishgard: The Tribunal Aethernet shard
    327767,  -- Ishgard: The Last Vigil Aethernet shard
    327770,  -- Idyllshire: West Idyllshire Aethernet shard

    -- Stormblood Aetherytes
    327792,  -- Kugane: Shiokaze Hostelry Aethernet shard
    327793,  -- Kugane: Pier #1 Aethernet shard
    327794,  -- Kugane: Thavnairian Consulate Aethernet shard
    327795,  -- Kugane: Kogane Dori Markets Aethernet shard
    327796,  -- Kugane: Bokairo Inn Aethernet shard
    327797,  -- Kugane: The Ruby Bazaar Aethernet shard
    327798,  -- Kugane: Sekiseigumi Barracks Aethernet shard
    327799,  -- Kugane: Rakuza District Aethernet shard
    327801,  -- Rhalgr's Reach: Western Rhalgr's Reach Aethernet shard
    327802,  -- Rhalgr's Reach: Northeastern Rhalgr's Reach Aethernet shard
    327805,  -- Ul'dah: Sapphire Avenue Exchange Aethernet shard
    327809,  -- The Doman Enclave: The Northern Enclave Aethernet shard
    327810,  -- The Doman Enclave: The Southern Enclave Aethernet shard

    -- Shadowbringers Aetherytes
    327815,  -- Eulmore: Southeast Derelicts Aethernet shard
    327829,  -- The Crystarium: Musica Universalis Markets Aethernet shard
    327830,  -- The Crystarium: Temenos Rookery Aethernet shard
    327831,  -- The Crystarium: The Dossal Gate Aethernet shard
    327832,  -- The Crystarium: The Pendants Aethernet shard
    327833,  -- The Crystarium: The Amaro Launch Aethernet shard
    327834,  -- The Crystarium: The Crystalline Mean Aethernet shard
    327835,  -- The Crystarium: The Cabinet of Curiosity Aethernet shard
    327837,  -- Eulmore: The Mainstay Aethernet shard
    327838,  -- Eulmore: Nightsoil Pots Aethernet shard
    327839,  -- Eulmore: The Glory Gate Aethernet shard
    327842,  -- The Doman Enclave: Ferry Docks Aethernet shard

    -- Endwalker Aetherytes
    327864,  -- Old Sharlayan: The Studium Aethernet shard
    327865,  -- Old Sharlayan: The Baldesion Annex Aethernet shard
    327866,  -- Old Sharlayan: The Rostrum Aethernet shard
    327867,  -- Old Sharlayan: The Leveilleur Estate Aethernet shard
    327868,  -- Old Sharlayan: Journey's End Aethernet shard
    327869,  -- Old Sharlayan: Scholar's Harbor Aethernet shard
    327871,  -- Radz-at-Han: Meghaduta Aethernet shard
    327872,  -- Radz-at-Han: Ruveydah Fibers Aethernet shard
    327873,  -- Radz-at-Han: Airship Landing Aethernet shard
    327874,  -- Radz-at-Han: Alzadaal's Peace Aethernet shard
    327875,  -- Radz-at-Han: The Hall of the Radiant Host Aethernet shard
    327876,  -- Radz-at-Han: Mehryde's Meyhane Aethernet shard
    327878,  -- Radz-at-Han: Kama Aethernet shard
    327879,  -- Radz-at-Han: The High Crucible of Al-Kimiya Aethernet shard

    -- Dawntrail Aetherytes
    327898,  -- Tuliyollal: Dirgible Landing Aethernet shard
    327899,  -- Tuliyollal: The Resplendent Quarter Aethernet shard
    327900,  -- Tuliyollal: The For'ard Cabins Aethernet shard
    327901,  -- Tuliyollal: Bayside Bevy Marketplace Aethernet shard
    327902,  -- Tuliyollal: Vollok Shoonsa Aethernet shard
    327904,  -- Tuliyollal: Brightploom Post Aethernet shard
    327910,  -- Solution Nine: Information Center Aethernet shard
    327911,  -- Solution Nine: True Vue Aethernet shard
    327912,  -- Solution Nine: Neon Stein Aethernet shard
    327913,  -- Solution Nine: The Arcadion Aethernet shard
    327914,  -- Solution Nine: Resolution Aethernet shard
    327915,  -- Solution Nine: Nexus Arcade Aethernet shard
    327916,  -- Solution Nine: Residential District Aethernet shard
}

-- TODO: Should probably break misc. events and their tables off into separate NPCs and objects eventually, but this is fine for now.
to_sort = {
    [720898] = "DeliveryMoogle.lua",
    [721096] = "ToyChest.lua",
    [721028] = "UnendingJourney.lua",
    [721226] = "Orchestrion.lua",
    [721347] = "GlamourDresser.lua",
    [721440] = "SummoningBell.lua",
    [720935] = "MarketBoard.lua",
    [720978] = "Armoire.lua",
    [1179657] = "Chocobokeep.lua", -- Chocobokeep in Central Shroud
}

-- Events in /common that aren't already get covered by other tables
common_events = {
    [720915] = "GenericMender.lua",
}

-- Not custom in the sense of non-SQEX content, just going based off the directory name
custom0_events = {
    [720916] = "cmndefinnbed_00020.lua",
}

custom1_events = {
    [721044] = "cmndefbeautysalon_00148.lua",
}

-- Events in quests/*
quests = {
    [1245185] = "OpeningLimsaLominsa.lua",
    [1245186] = "OpeningGridania.lua",
    [1245187] = "OpeningUldah.lua", 
}

COMMON_DIR = "events/common/"
WARP_DIR = "events/warp/"
TOSORT_DIR = "events/tosort/"
OPENING_DIR = "events/quest/opening/"
CUSTOM0_DIR = "events/custom/000/"
CUSTOM1_DIR = "events/custom/001/"

for _, event_id in pairs(generic_warps) do
    registerEvent(event_id, "events/common/GenericWarp.lua")
end

for _, event_id in pairs(generic_inns) do
    registerEvent(event_id, "events/warp/WarpInnGeneric.lua" )
end

for _, event_id in pairs(generic_aetherytes) do
    registerEvent(event_id, "events/common/GenericAetheryte.lua")
end

for _, event_id in pairs(generic_anetshards) do
    registerEvent(event_id, "events/common/GenericAethernetShard.lua")
end

for event_id, script_file in pairs(to_sort) do
    registerEvent(event_id, TOSORT_DIR..script_file)
end

for event_id, script_file in pairs(common_events) do
    registerEvent(event_id, COMMON_DIR..script_file)
end

for event_id, script_file in pairs(custom0_events) do
    registerEvent(event_id, CUSTOM0_DIR..script_file)
end

for event_id, script_file in pairs(custom1_events) do
    registerEvent(event_id, CUSTOM1_DIR..script_file)
end

for event_id, script_file in pairs(quests) do
    registerEvent(event_id, OPENING_DIR..script_file)
end
