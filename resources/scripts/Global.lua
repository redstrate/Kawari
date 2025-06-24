function onBeginLogin(player)
    -- send a welcome message
    player:send_message("Welcome to Kawari!")
end

function onCommandRequiredRankInsufficientError(player)
    player:send_message("You do not have permission to run this command.")
end

function onCommandRequiredRankMissingError(additional_information, player)
    local error_msg = "Your script does not define the required_rank variable. Please define it in your script for it to run."

    player:send_message(string.format("%s\nAdditional information: %s", error_msg, additional_information))
end

function onUnknownCommandError(command_name, player)
    player:send_message(string.format("Unknown command %s", command_name))
end

function split(input, separator)
    if separator == nil then
        separator = '%s'
    end

    local t = {}
    for str in string.gmatch(input, '([^'..separator..']+)') do
        table.insert(t, str)
    end

    return t
end

-- Constants
GM_RANK_NORMALUSER = 0
GM_RANK_GAMEMASTER = 1
GM_RANK_EVENTJUNIOR = 3
GM_RANK_EVENTSENIOR = 4
GM_RANK_SUPPORT = 5
GM_RANK_SENIOR = 7
GM_RANK_DEBUG = 90
GM_RANK_MAX = 255 -- Doesn't exist, used for purposes of testing permissions in scripts

-- please keep these ids sorted!

-- Actions
registerAction(3, "actions/Sprint.lua")
registerAction(5, "actions/Teleport.lua")
registerAction(9, "actions/FastBlade.lua")

-- Items
registerAction(6221, "items/Fantasia.lua")

-- Events
registerEvent(131077, "common/GenericWarp.lua") -- Ferry Skipper from Old Gridania to East Shroud: Sweetbloom Pier
registerEvent(131078, "common/GenericWarp.lua") -- Ferry Skipper from East Shroud: Sweetbloom Pier to Old Gridania
registerEvent(131079, "warp/WarpInnGeneric.lua") -- Exit Limsa Upper Decks to Mizzenmast Inn room
registerEvent(131080, "warp/WarpInnGeneric.lua") -- Exit New Gridania to The Roost Inn room
registerEvent(131081, "warp/WarpInnGeneric.lua") -- Exit Ul'dah: Steps of Nald to The Hourglass Inn room
registerEvent(131082, "common/GenericWarp.lua") -- Exit Mizzenmast Inn to Limsa Upper Decks
registerEvent(131083, "common/GenericWarp.lua") -- Exit The Roost to New Gridania
registerEvent(131084, "common/GenericWarp.lua") -- Exit The Hourglass to Ul'dah: Steps of Nald
registerEvent(131092, "common/GenericWarp.lua") -- Exit from Limsa Bulwark Hall and/or Drowning Wench to Airship Landing
registerEvent(131093, "common/GenericWarp.lua") -- Exit from Limsa Bullwark Hall and/or Airship Landing to Drowning Wench
registerEvent(131094, "common/GenericWarp.lua") -- Exit from Limsa Airship Landing and/or Drowning Wench to Bulwark Hall
registerEvent(131095, "common/GenericWarp.lua") -- Exit from Ul'dah Hustings Strip and/or Ruby Road Exchange to Airship Landing, these three events get reused in several places to ensure thay all connect
registerEvent(131096, "common/GenericWarp.lua") -- Exit from Ul'dah Airship Landing and/or Ruby Road Exchange to Hustings Strip
registerEvent(131097, "common/GenericWarp.lua") -- Exit from Ul'dah Airship Landing and/or Husting Strip to Ruby Road Exchange
--registerEvent(131113, "common/GenericWarp.lua") -- (currently doesn't react, seems to need different handling) Ferry Skipper from Upper La Noscea: Memeroon's Trading Post to Upper La Noscea: Jijiroon's Trading Post
--registerEvent(131114, "common/GenericWarp.lua") -- (currently doesn't react, seems to need different handling) Ferry Skipper from Upper La Noscea: Jijiroon's Trading Post to Upper La Noscea: Memeroon's Trading Post
registerEvent(131126, "common/GenericWarp.lua") -- Gatekeeper from Southern Thanalan: Nald's Reflection to Southern Thanalan: The Minotaur Malm
--registerEvent(131158, None) -- Ferry Skipper from Old Gridania to The Lavender Beds, needs special handling for housing
--registerEvent(131169, "common/GenericWarp.lua") -- Ferry Skipper from Eastern La Noscea: Costa Del Sol to ELN: Rhotano Privateer (currently broken)
registerEvent(131195, "common/GenericWarp.lua") -- Exit from Fortemps manor to Ishgard: The Pillars
registerEvent(131204, "warp/WarpInnGeneric.lua") -- Exit Ishgard: Foundation to Cloud Nine Inn room
registerEvent(131205, "common/GenericWarp.lua") -- Exit Cloud Nine to Ishgard: Foundation
registerEvent(131245, "warp/WarpInnGeneric.lua") -- Exit Kugane to Bokairo Inn room
registerEvent(131246, "common/GenericWarp.lua") -- Exit Bokairo Inn to Kugane
registerEvent(131250, "common/GenericWarp.lua") -- Gatekeeper from The Fringes: Castrum Oriens to East Shroud: Amarissaaix's Spire
registerEvent(131266, "common/GenericWarp.lua") -- Gatekeeper from The House of the Fierce to dead-end cave (unable to dive currently)
registerEvent(131268, "common/GenericWarp.lua") -- Enclave Skiff Captain from The Doman Enclave to Yanxia: The Glittering Basin
registerEvent(131312, "common/GenericWarp.lua") -- Exit The Pendants Personal Suite to Crystarium
registerEvent(131402, "common/GenericWarp.lua") -- Exit Andron to Old Sharlayan
registerEvent(131428, "common/GenericWarp.lua") -- Mothercrystal Exit (note: warp doesn't work for some reason?)
registerEvent(131519, "common/GenericWarp.lua") -- Faire Adventurer from Eastern La Noscea: bottom of the Moonfire Festival (2023) tower to the first tier of the tower
registerEvent(131578, "common/GenericWarp.lua") -- Exit The For'ard Cabins to Tuliyollal

-- A Realm Reborn Aetherytes
registerEvent(327683, "common/GenericAetheryte.lua") -- Bentbranch Meadows Aetheryte
registerEvent(327684, "common/GenericAetheryte.lua") -- The Hawthorne Hut Aetheryte
registerEvent(327685, "common/GenericAetheryte.lua") -- Quarrymill Aetheryte
registerEvent(327686, "common/GenericAetheryte.lua") -- Camp Tranquil Aetheryte
registerEvent(327687, "common/GenericAetheryte.lua") -- Fallgourd Float Aetheryte
registerEvent(327690, "common/GenericAetheryte.lua") -- Moraby Drydocks Aetheryte
registerEvent(327691, "common/GenericAetheryte.lua") -- Costa del Sol Aetheryte
registerEvent(327692, "common/GenericAetheryte.lua") -- Wineport Aetheryte
registerEvent(327693, "common/GenericAetheryte.lua") -- Swiftperch Aetheryte
registerEvent(327694, "common/GenericAetheryte.lua") -- Aleport Aetheryte
registerEvent(327695, "common/GenericAetheryte.lua") -- Camp Bronze Lake Aetheryte
registerEvent(327696, "common/GenericAetheryte.lua") -- Camp Overlook Aetheryte
registerEvent(327697, "common/GenericAetheryte.lua") -- Horizon Aetheryte
registerEvent(327698, "common/GenericAetheryte.lua") -- Camp Drybone Aetheryte
registerEvent(327699, "common/GenericAetheryte.lua") -- Little Ala Mhigo Aetheryte
registerEvent(327700, "common/GenericAetheryte.lua") -- Forgotten Springs Aetheryte
registerEvent(327701, "common/GenericAetheryte.lua") -- Camp Bluefog Aetheryte
registerEvent(327702, "common/GenericAetheryte.lua") -- Ceruleum Processing Plant Aetheryte
registerEvent(327703, "common/GenericAetheryte.lua") -- Camp Dragonhead Aetheryte
registerEvent(327705, "common/GenericAethernetShard.lua") -- Gridania: Archers' Guild Aethernet shard
registerEvent(327706, "common/GenericAethernetShard.lua") -- Gridania: Leatherworkers' Guild & Shaded Bower Aethernet shard
registerEvent(327707, "common/GenericAethernetShard.lua") -- Gridania: Lancers' Guild Aethernet shard
registerEvent(327708, "common/GenericAethernetShard.lua") -- Gridania: Conjurer' Guild Aethernet shard
registerEvent(327709, "common/GenericAethernetShard.lua") -- Gridania: Botanists' Guild Aethernet shard
registerEvent(327710, "common/GenericAethernetShard.lua") -- Gridania: Mih Khetto's Amphitheatre Aethernet shard
registerEvent(327713, "common/GenericAethernetShard.lua") -- Ul'dah: Adventurers' Guild Aethernet shard
registerEvent(327714, "common/GenericAethernetShard.lua") -- Ul'dah: Thaumaturges' Guild Aethernet shard
registerEvent(327715, "common/GenericAethernetShard.lua") -- Ul'dah: Gladiators' Guild Aethernet shard
registerEvent(327716, "common/GenericAethernetShard.lua") -- Ul'dah: Miners' Guild Aethernet shard
registerEvent(327717, "common/GenericAethernetShard.lua") -- Ul'dah: Alchemists' Guild Aethernet shard
registerEvent(327721, "common/GenericAethernetShard.lua") -- Limsa Lominsa: The Aftcastle Aethernet shard
registerEvent(327722, "common/GenericAethernetShard.lua") -- Limsa Lominsa: Culinarians' Guild Aethernet shard
registerEvent(327723, "common/GenericAethernetShard.lua") -- Limsa Lominsa: Arcanists' Guild Aethernet shard
registerEvent(327724, "common/GenericAethernetShard.lua") -- Limsa Lominsa: Fishermen's Guild Aethernet shard
registerEvent(327727, "common/GenericAethernetShard.lua") -- Ul'dah: Weaver's Guild Aethernet shard
registerEvent(327728, "common/GenericAethernetShard.lua") -- Limsa Lominsa: Marauders' Guild Aethernet shard
registerEvent(327729, "common/GenericAethernetShard.lua") -- Limsa Lominsa: Hawker's Alley Aethernet shard
registerEvent(327730, "common/GenericAethernetShard.lua") -- Ul'dah: Goldsmith's Guild Aethernet shard
registerEvent(327731, "common/GenericAethernetShard.lua") -- Ul'dah: The Chamber of Rule Aethernet shard
registerEvent(327732, "common/GenericAetheryte.lua") -- Summerford Farms Aetheryte
registerEvent(327733, "common/GenericAetheryte.lua") -- Black Brush Station Aetheryte
registerEvent(327735, "common/GenericAetheryte.lua") -- Wolves' Den Pier Aetheryte
-- registerevent(327???, "common/GenericAetheryte.lua") -- Ul'dah: Steps of Nald Aetheryte, currently unknown due to the entity not spawning
-- registerevent(327???, "common/GenericAetheryte.lua") -- Limsa Lominsa: The Lower Decks Aetheryte, currently unknown due to the entity not spawning
registerEvent(327742, "common/GenericAetheryte.lua") -- The Gold Saucer Aetheryte

-- Heavensward Aetherytes
registerEvent(327750, "common/GenericAetheryte.lua") -- Ishgard: Foundation Aetheryte
registerEvent(327751, "common/GenericAetheryte.lua") -- Falcon's Nest Aetheryte
registerEvent(327752, "common/GenericAetheryte.lua") -- Camp Cloudtop Aetheryte
registerEvent(327753, "common/GenericAetheryte.lua") -- Ok' Zundu Aetheryte
registerEvent(327754, "common/GenericAetheryte.lua") -- Helix Aetheryte
registerEvent(327755, "common/GenericAetheryte.lua") -- Idyllshire Aetheryte
registerEvent(327756, "common/GenericAetheryte.lua") -- Tailfeather Aetheryte
registerEvent(327757, "common/GenericAetheryte.lua") -- Anyx Trine Aetheryte
registerEvent(327758, "common/GenericAetheryte.lua") -- Moghome Aetheryte
registerEvent(327759, "common/GenericAetheryte.lua") -- Zenith Aetheryte
registerEvent(327760, "common/GenericAethernetShard.lua") -- Ishgard: The Forgotten Knight Aethernet shard
registerEvent(327761, "common/GenericAethernetShard.lua") -- Ishgard: Skysteel Manufactory Aethernet shard
registerEvent(327762, "common/GenericAethernetShard.lua") -- Ishgard: The Brume Aethernet shard
registerEvent(327763, "common/GenericAethernetShard.lua") -- Ishgard: Anathaeum Astrologicum Aethernet shard
registerEvent(327764, "common/GenericAethernetShard.lua") -- Ishgard: The Jewled Crozier Aethernet shard
registerEvent(327765, "common/GenericAethernetShard.lua") -- Ishgard: Saint Reymanaud's Cathedral Aethernet shard
registerEvent(327766, "common/GenericAethernetShard.lua") -- Ishgard: The Tribunal Aethernet shard
registerEvent(327767, "common/GenericAethernetShard.lua") -- Ishgard: The Last Vigil Aethernet shard
registerEvent(327770, "common/GenericAethernetShard.lua") -- Idyllshire: West Idyllshire Aethernet shard

-- Stormblood Aetherytes
registerEvent(327778, "common/GenericAetheryte.lua") -- Castrum Oriens Aetheryte
registerEvent(327779, "common/GenericAetheryte.lua") -- The Peering Stones Aetheryte
registerEvent(327780, "common/GenericAetheryte.lua") -- Ala Gannha Aetheryte
registerEvent(327781, "common/GenericAetheryte.lua") -- Ala Ghiri Aetheryte
registerEvent(327782, "common/GenericAetheryte.lua") -- Porta Praetoria Aetheryte
registerEvent(327783, "common/GenericAetheryte.lua") -- The Ala Mhigan Quarter Aetheryte
registerEvent(327784, "common/GenericAetheryte.lua") -- Rhalgr's Reach Aetheryte
registerEvent(327785, "common/GenericAetheryte.lua") -- Tamamizu Aetheryte
registerEvent(327786, "common/GenericAetheryte.lua") -- Onokoro Aetheryte
registerEvent(327787, "common/GenericAetheryte.lua") -- Namai Aetheryte
registerEvent(327788, "common/GenericAetheryte.lua") -- The House of the Fierce Aetheryte
registerEvent(327789, "common/GenericAetheryte.lua") -- Reunion Aetheryte
registerEvent(327790, "common/GenericAetheryte.lua") -- The Dawn Throne Aetheryte
registerEvent(327791, "common/GenericAetheryte.lua") -- Kugane Aetheryte
registerEvent(327792, "common/GenericAethernetShard.lua") -- Kugane: Shiokaze Hostelry Aethernet shard
registerEvent(327793, "common/GenericAethernetShard.lua") -- Kugane: Pier #1 Aethernet shard
registerEvent(327794, "common/GenericAethernetShard.lua") -- Kugane: Thavnairian Consulate Aethernet shard
registerEvent(327795, "common/GenericAethernetShard.lua") -- Kugane: Kogane Dori Markets Aethernet shard
registerEvent(327796, "common/GenericAethernetShard.lua") -- Kugane: Bokairo Inn Aethernet shard
registerEvent(327797, "common/GenericAethernetShard.lua") -- Kugane: The Ruby Bazaar Aethernet shard
registerEvent(327798, "common/GenericAethernetShard.lua") -- Kugane: Sekiseigumi Barracks Aethernet shard
registerEvent(327799, "common/GenericAethernetShard.lua") -- Kugane: Rakuza District Aethernet shard
registerEvent(327801, "common/GenericAethernetShard.lua") -- Rhalgr's Reach: Western Rhalgr's Reach Aethernet shard
registerEvent(327802, "common/GenericAethernetShard.lua") -- Rhalgr's Reach: Northeastern Rhalgr's Reach Aethernet shard
registerEvent(327805, "common/GenericAethernetShard.lua") -- Ul'dah: Sapphire Avenue Exchange Aethernet shard
registerEvent(327807, "common/GenericAetheryte.lua") -- The Doman Enclave Aetheryte
registerEvent(327808, "common/GenericAetheryte.lua") -- Dhoro Iloh Aetheryte
registerEvent(327809, "common/GenericAethernetShard.lua") -- The Doman Enclave: The Northern Enclave Aethernet shard
registerEvent(327810, "common/GenericAethernetShard.lua") -- The Doman Enclave: The Southern Enclave Aethernet shard

-- Shadowbringers Aetherytes
registerEvent(327812, "common/GenericAetheryte.lua") -- Fort Jobb Aetheryte
registerEvent(327814, "common/GenericAetheryte.lua") -- Eulmore Aetheryte
registerEvent(327815, "common/GenericAethernetShard.lua") -- Eulmore: Southeast Derelicts Aethernet shard
registerEvent(327816, "common/GenericAetheryte.lua") -- The Ostal Imperative Aetheryte
registerEvent(327817, "common/GenericAetheryte.lua") -- Stilltide Aetheryte
registerEvent(327818, "common/GenericAetheryte.lua") -- Wright Aetheryte
registerEvent(327819, "common/GenericAetheryte.lua") -- Tomra Aetheryte
registerEvent(327820, "common/GenericAetheryte.lua") -- Mord Souq Aetheryte
registerEvent(327821, "common/GenericAetheryte.lua") -- Twine Aetheryte
registerEvent(327822, "common/GenericAetheryte.lua") -- Slitherbough Aetheryte
registerEvent(327823, "common/GenericAetheryte.lua") -- Fanow Aetheryte
registerEvent(327824, "common/GenericAetheryte.lua") -- Lydha Lran Aetheryte
registerEvent(327825, "common/GenericAetheryte.lua") -- Pla Enni Aetheryte
registerEvent(327826, "common/GenericAetheryte.lua") -- Wolekdorf Aetheryte
registerEvent(327827, "common/GenericAetheryte.lua") -- The Ondo Cups Aetheryte
registerEvent(327828, "common/GenericAetheryte.lua") -- The Macarenses Angle Aetheryte
registerEvent(327829, "common/GenericAethernetShard.lua") -- The Crystarium: Musica Universalis Markets Aethernet shard
registerEvent(327830, "common/GenericAethernetShard.lua") -- The Crystarium: Temenos Rookery Aethernet shard
registerEvent(327831, "common/GenericAethernetShard.lua") -- The Crystarium: The Dossal Gate Aethernet shard
registerEvent(327832, "common/GenericAethernetShard.lua") -- The Crystarium: The Pendants Aethernet shard
registerEvent(327833, "common/GenericAethernetShard.lua") -- The Crystarium: The Amaro Launch Aethernet shard
registerEvent(327834, "common/GenericAethernetShard.lua") -- The Crystarium: The Crystalline Mean Aethernet shard
registerEvent(327835, "common/GenericAethernetShard.lua") -- The Crystarium: The Cabinet of Curiosity Aethernet shard
registerEvent(327837, "common/GenericAethernetShard.lua") -- Eulmore: The Mainstay Aethernet shard
registerEvent(327838, "common/GenericAethernetShard.lua") -- Eulmore: Nightsoil Pots Aethernet shard
registerEvent(327839, "common/GenericAethernetShard.lua") -- Eulmore: The Glory Gate Aethernet shard
registerEvent(327841, "common/GenericAetheryte.lua") -- The Inn at Journey's Head Aetheryte
registerEvent(327842, "common/GenericAethernetShard.lua") -- The Doman Enclave: Ferry Docks Aethernet shard
-- registerEvent(3278??, "common/GenericAetheryte.lua") -- The Crystarium Aetheryte, currently unknown due to the entity not spawning

-- Endwalker Aetherytes
registerEvent(327846, "common/GenericAetheryte.lua") -- The Archeion Aetheryte
registerEvent(327847, "common/GenericAetheryte.lua") -- Sharlayan Hamlet Aetheryte
registerEvent(327848, "common/GenericAetheryte.lua") -- Aporia Aetheryte
registerEvent(327849, "common/GenericAetheryte.lua") -- Yedlihmad Aetheryte
registerEvent(327850, "common/GenericAetheryte.lua") -- The Great Work Aetheryte
registerEvent(327851, "common/GenericAetheryte.lua") -- Palaka's Stand Aetheryte
registerEvent(327852, "common/GenericAetheryte.lua") -- Camp Broken Glass Aetheryte
registerEvent(327853, "common/GenericAetheryte.lua") -- Tertium Aetheryte
registerEvent(327854, "common/GenericAetheryte.lua") -- Sinus Lacrimarum Aetheryte
registerEvent(327855, "common/GenericAetheryte.lua") -- Bestways Burrow Aetheryte
registerEvent(327856, "common/GenericAetheryte.lua") -- Anagnorisis Aetheryte
registerEvent(327857, "common/GenericAetheryte.lua") -- The Twelve Wonders Aetheryte
registerEvent(327858, "common/GenericAetheryte.lua") -- Poieten Oikos Aetheryte
registerEvent(327859, "common/GenericAetheryte.lua") -- Reah Tahra Aetheryte
registerEvent(327860, "common/GenericAetheryte.lua") -- Abode of the Ea Aetheryte
registerEvent(327861, "common/GenericAetheryte.lua") -- Base Omicron Aetheryte
registerEvent(327862, "common/GenericAetheryte.lua") -- Old Sharlayan Aetheryte
registerEvent(327863, "common/GenericAetheryte.lua") -- Radz-at-Han Aetheryte
registerEvent(327864, "common/GenericAethernetShard.lua") -- Old Sharlayan: The Studium Aethernet shard
registerEvent(327865, "common/GenericAethernetShard.lua") -- Old Sharlayan: The Baldesion Annex Aethernet shard
registerEvent(327866, "common/GenericAethernetShard.lua") -- Old Sharlayan: The Rostrum Aethernet shard
registerEvent(327867, "common/GenericAethernetShard.lua") -- Old Sharlayan: The Leveilleur Estate Aethernet shard
registerEvent(327868, "common/GenericAethernetShard.lua") -- Old Sharlayan: Journey's End Aethernet shard
registerEvent(327869, "common/GenericAethernetShard.lua") -- Old Sharlayan: Scholar's Harbor Aethernet shard
registerEvent(327871, "common/GenericAethernetShard.lua") -- Radz-at-Han: Meghaduta Aethernet shard
registerEvent(327872, "common/GenericAethernetShard.lua") -- Radz-at-Han: Ruveydah Fibers Aethernet shard
registerEvent(327873, "common/GenericAethernetShard.lua") -- Radz-at-Han: Airship Landing Aethernet shard
registerEvent(327874, "common/GenericAethernetShard.lua") -- Radz-at-Han: Alzadaal's Peace Aethernet shard
registerEvent(327875, "common/GenericAethernetShard.lua") -- Radz-at-Han: The Hall of the Radiant Host Aethernet shard
registerEvent(327876, "common/GenericAethernetShard.lua") -- Radz-at-Han: Mehryde's Meyhane Aethernet shard
registerEvent(327878, "common/GenericAethernetShard.lua") -- Radz-at-Han: Kama Aethernet shard
registerEvent(327879, "common/GenericAethernetShard.lua") -- Radz-at-Han: The High Crucible of Al-Kimiya Aethernet shard

-- Dawntrail Aetherytes
registerEvent(327880, "common/GenericAetheryte.lua") -- Wachunpelo Aetheryte
registerEvent(327881, "common/GenericAetheryte.lua") -- Worlar's Echo Aetheryte
registerEvent(327882, "common/GenericAetheryte.lua") -- Ok'hanu Aetheryte
registerEvent(327883, "common/GenericAetheryte.lua") -- Many Fires Aetheryte
registerEvent(327884, "common/GenericAetheryte.lua") -- Earthenshire Aetheryte
registerEvent(327885, "common/GenericAetheryte.lua") -- Iq Br'aax Aetheryte
registerEvent(327886, "common/GenericAetheryte.lua") -- Mamook Aetheryte
registerEvent(327887, "common/GenericAetheryte.lua") -- Hhusatahwi Aetheryte
registerEvent(327888, "common/GenericAetheryte.lua") -- Sheshenewezi Springs Aetheryte
registerEvent(327889, "common/GenericAetheryte.lua") -- Mehwahhetsoan Aetheryte
registerEvent(327890, "common/GenericAetheryte.lua") -- Yyasulani Station Aetheryte
registerEvent(327891, "common/GenericAetheryte.lua") -- The Outskirts Aetheryte
registerEvent(327892, "common/GenericAetheryte.lua") -- Electrope Strike Aetheryte
registerEvent(327893, "common/GenericAetheryte.lua") -- Leynode Mnemo Aetheryte
registerEvent(327894, "common/GenericAetheryte.lua") -- Leynode Pyro Aetheryte
registerEvent(327895, "common/GenericAetheryte.lua") -- Leynode Aero Aetheryte
registerEvent(327896, "common/GenericAetheryte.lua") -- Tuliyollal Aetheryte
registerEvent(327897, "common/GenericAetheryte.lua") -- Solution Nine Aetheryte
registerEvent(327898, "common/GenericAethernetShard.lua") -- Tuliyollal: Dirgible Landing Aethernet shard
registerEvent(327899, "common/GenericAethernetShard.lua") -- Tuliyollal: The Resplendent Quarter Aethernet shard
registerEvent(327900, "common/GenericAethernetShard.lua") -- Tuliyollal: The For'ard Cabins Aethernet shard
registerEvent(327901, "common/GenericAethernetShard.lua") -- Tuliyollal: Bayside Bevy Marketplace Aethernet shard
registerEvent(327902, "common/GenericAethernetShard.lua") -- Tuliyollal: Vollok Shoonsa Aethernet shard
registerEvent(327904, "common/GenericAethernetShard.lua") -- Tuliyollal: Brightploom Post Aethernet shard
registerEvent(327910, "common/GenericAethernetShard.lua") -- Solution Nine: Information Center Aethernet shard
registerEvent(327911, "common/GenericAethernetShard.lua") -- Solution Nine: True Vue Aethernet shard
registerEvent(327912, "common/GenericAethernetShard.lua") -- Solution Nine: Neon Stein Aethernet shard
registerEvent(327913, "common/GenericAethernetShard.lua") -- Solution Nine: The Arcadion Aethernet shard
registerEvent(327914, "common/GenericAethernetShard.lua") -- Solution Nine: Resolution Aethernet shard
registerEvent(327915, "common/GenericAethernetShard.lua") -- Solution Nine: Nexus Arcade Aethernet shard
registerEvent(327916, "common/GenericAethernetShard.lua") -- Solution Nine: Residential District Aethernet shard
registerEvent(327918, "common/GenericAetheryte.lua") -- Dock Poga Aetheryte

-- Misc. Events
registerEvent(720898, "tosort/DeliveryMoogle.lua")
registerEvent(720915, "common/GenericMender.lua")
registerEvent(720916, "custom/000/cmndefinnbed_00020.lua")
registerEvent(721096, "tosort/ToyChest.lua")
registerEvent(721028, "tosort/UnendingJourney.lua")
registerEvent(721044, "tosort/CrystalBell.lua")
registerEvent(721226, "tosort/Orchestrion.lua")
registerEvent(721347, "tosort/GlamourDresser.lua")
registerEvent(721440, "tosort/SummoningBell.lua")
registerEvent(720935, "tosort/MarketBoard.lua")
registerEvent(720978, "tosort/Armoire.lua")
registerEvent(1179657, "tosort/Chocobokeep.lua") -- Chocobokeep in Central Shroud
registerEvent(1245185, "opening/OpeningLimsaLominsa.lua")
registerEvent(1245186, "opening/OpeningGridania.lua")
registerEvent(1245187, "opening/OpeningUldah.lua")

-- TODO: Generic warps might be decided through ArrayEventHandler?

-- Commands
registerCommand("setpos", "commands/debug/SetPos.lua")
registerCommand("classjob", "commands/debug/ClassJob.lua")
registerCommand("setspeed", "commands/debug/SetSpeed.lua")
registerCommand("nudge", "commands/debug/Nudge.lua")
registerCommand("festival", "commands/debug/Festival.lua")
registerCommand("permtest", "commands/debug/PermissionTest.lua")
registerCommand("unlock", "commands/debug/Unlock.lua")
registerCommand("wireframe", "commands/debug/ToggleWireframe.lua")
registerCommand("invis", "commands/debug/ToggleInvisibility.lua")
registerCommand("unlockaetheryte", "commands/debug/UnlockAetheryte.lua")
registerCommand("teri", "commands/debug/ChangeTerritory.lua")
