-- Please keep these ids sorted in each table!

-- TODO: Generic warps might be decided through ArrayEventHandler?
generic_warps = {
    131077,  -- Ferry Skipper from Old Gridania to East Shroud: Sweetbloom Pier
    131078,  -- Ferry Skipper from East Shroud: Sweetbloom Pier to Old Gridania
    131079,  -- Exit Limsa Upper Decks to Mizzenmast Inn room
    131080,  -- Exit New Gridania to The Roost Inn room
    131081,  -- Exit Ul'dah: Steps of Nald to The Hourglass Inn room
    131082,  -- Exit Mizzenmast Inn to Limsa Upper Decks
    131083,  -- Exit The Roost to New Gridania
    131084,  -- Exit The Hourglass to Ul'dah: Steps of Nald
    131086,  -- Ferry Skipper from Western Thanalan: The Silver Bazaar to Western Thanalan: Cescent Cove
    131087,  -- Ferry Skipper from Western Thanalan: Crescent Cove to Western Thanalan: The Silver Bazaar
    131088,  -- Exit from Western Thanalan: Vesper Bay to The Waking Sands
    131089,  -- Exit from The Waking Sands to Western Thanalan: Vesper Bay
    131090,  -- Exit from The Waking Sands to The Solar
    131091,  -- Exit from The Solar to The Waking Sands
    131092,  -- Exit from Limsa Bulwark Hall and/or Drowning Wench to Airship Landing
    131093,  -- Exit from Limsa Bullwark Hall and/or Airship Landing to Drowning Wench
    131094,  -- Exit from Limsa Airship Landing and/or Drowning Wench to Bulwark Hall
    131095,  -- Exit from Ul'dah Hustings Strip and/or Ruby Road Exchange to Airship Landing, these three events get reused in several places to ensure they all connect
    131096,  -- Exit from Ul'dah Airship Landing and/or Ruby Road Exchange to Hustings Strip
    131097,  -- Exit from Ul'dah Airship Landing and/or Husting Strip to Ruby Road Exchange
    131107,  -- Nunuri <Ferry Ticketer> from Western Thanalan: Vesper Bay to Limsa Lominsa: The Lower Decks
    131108,  -- Rhetkympf <Ferry Ticketer> from Limsa Lominsa: The Lower Decks to Western Thanalan: Vesper Bay
    131109,  -- Rerenasu <Ferry Skipper> from Limsa Lominsa: The Lower Decks to Western La Noscea: Aleport
    131110,  -- Ferry Skipper from Western La Noscea: Aleport to Limsa Lominsa: The Lower Decks
    131111,  -- Rerenasu <Ferry Skipper> from Limsa Lominsa: The Lower Decks to Eastern La Noscea: Costa Del Sol
    131112,  -- Ferry Skipper from Eastern La Noscea: Costa Del Sol to Limsa Lominsa: The Lower Decks
    131113,  -- Ferry Skipper from Upper La Noscea: Memeroon's Trading Post to Upper La Noscea: Jijiroon's Trading Post
    131114,  -- Ferry Skipper from Upper La Noscea: Jijiroon's Trading Post to Upper La Noscea: Memeroon's Trading Post
    131115,  -- O'nolosi <Ferry Skipper> from Lower La Noscea: Candlekeep Quay to Western La Noscea: Aleport
    131116,  -- Ferry Skipper from Western La Noscea: Aleport to Lower La Noscea: Candlekeep Quay
    131119,  -- Ferry Skipper from Eastern La Noscea: Hidden Falls Docks to Eastern La Noscea: Raincatcher Gully Docks
    131120,  -- Ferry Skipper from Eastern La Noscea: Raincatcher Gully Docks to Eastern La Noscea: Hidden Falls Docks
    131126,  -- Gatekeeper from Southern Thanalan: Nald's Reflection to Southern Thanalan: The Minotaur Malm
    131131,  -- Ferry Skipper from Moraby Drydocks to Wolves' Den Pier
    131132,  -- Ferry Skipper from Wolves' Den Pier to Moraby Drydocks
    131133,  -- Ferry Skipper from Western La Noscea: The Isles of Umbra to Western La Noscea: Aleport
    131134,  -- Ferry Skipper from Western La Noscea: Aleport to Western La Noscea: The Isles of Umbra
    -- 131158, None -- Ferry Skipper from Old Gridania to The Lavender Beds, needs special handling for housing
    -- 131160, None -- Rerenasu <Ferry Skipper> from Limsa Lominsa: The Lower Decks to Mist, needs special handling for housing
    131169,  -- Ferry Skipper from Eastern La Noscea: Costa Del Sol to ELN: Rhotano Privateer
    131177,  -- Exit from The Gold Saucer (Lift Operator) to The Gold Saucer: Chocobo Square
    131178,  -- Exit from The Gold Saucer: Chocobo Square (Lift Operator) to The Gold Saucer
    131192,  -- House Fortemps Guard <Gatekeep> From Ishgard: The Pillars to Fortemps Manor
    131195,  -- Exit from Fortemps manor to Ishgard: The Pillars
    131204,  -- Exit Ishgard: Foundation to Cloud Nine Inn room
    131205,  -- Exit Cloud Nine to Ishgard: Foundation
    131245,  -- Exit Kugane to Bokairo Inn room
    131246,  -- Exit Bokairo Inn to Kugane
    -- 131248,  -- Kimachi <Ferry Skipper> from Kugane to Shirogane, needs special handling for housing
    131250,  -- Gatekeeper from The Fringes: Castrum Oriens to East Shroud: Amarissaaix's Spire
    131251,  -- Gatekeeper from East Shroud: Amarissaaix's Spire to The Fringes: Castrum Oriens
    131252,  -- Uguisu <Ferry Skipper> from Kugane to Limsa Lominsa: The Lower Decks
    131253,  -- East Aldenard Trading Company Sailor from Limsa Lominsa: The Lower Decks to Kugane
    131255,  -- Ala Mhigan Resistance Gate Guard from The Fringes: Virdjala to The Fringes: Pike Falls
    131266,  -- Gatekeeper from The House of the Fierce to dead-end cave (unable to dive currently)
    131268,  -- Enclave Skiff Captain from The Doman Enclave to Yanxia: The Glittering Basin
    131299,  -- Ala Mhigan Resistance Gate Guard from The Fringes: Pike Falls to The Fringes: Virdjala
    131312,  -- Exit The Pendants Personal Suite to Crystarium
    131313,  -- Exit from The Crown Lift (Lift Operator) to Eulmore: The Canopy
    131390,  -- Exit via Pawlin <Dreamer's Run Doorman> from Old Gridania: Dreamer's Run (old Hatchingtide event area which is now out of bounds) to Old Gridania: Botanists' Guild
    131402,  -- Exit Andron to Old Sharlayan
    131405,  -- Aergwynt <Ferry Ticketer> from Old Sharlayan to Limsa Lominsa: The Lower Decks
    131406,  -- Sailor <Ferryman> from Limsa Lominsa: The Lower Decks to Old Sharlayan
    131428,  -- Exit from The Mothercrystal to Labyrinthos: The Aitiascope (outside, on overworld): probably supposed to drop you into a cutscene zone instead.
    131519,  -- Faire Adventurer from Eastern La Noscea: bottom of the Moonfire Festival (2023 tower to the first tier of the tower
    131545,  -- Port Official from Tuliyollal to Old Sharlayan
    131578,  -- Exit The For'ard Cabins to Tuliyollal
    131609,  -- Exit from The Ageless Necropolis to Living Memory: The Meso Terminal
}

generic_inns = {
    131079, -- Exit Limsa Upper Decks to Mizzenmast Inn room
    131080, -- Exit New Gridania to The Roost Inn room
    131081, -- Exit Ul'dah: Steps of Nald to The Hourglass Inn room
    131204, -- Exit Ishgard: Foundation to Cloud Nine Inn room
    131245, -- Exit Kugane to Bokairo Inn room
    131316, -- Exit from The Crystarium to The Pendants Personal Suite
    131401, -- Exit from Old Sharlayan to The Andron, this does not work currently because the actor doesn't spawn
    131576, -- Exit from Tuliyollal to The For'ard Cabins, this does not work currently because the actor doesn't spawn
}

generic_aetherytes = {
    -- A Realm Reborn Aetherytes
    327682, -- New Gridania Aetheryte, currently not working due to the aetheryte actor not spawning
    327683,  -- Bentbranch Meadows Aetheryte
    327684,  -- The Hawthorne Hut Aetheryte
    327685,  -- Quarrymill Aetheryte
    327686,  -- Camp Tranquil Aetheryte
    327687,  -- Fallgourd Float Aetheryte
    327688,  -- Limsa Lominsa: The Lower Decks Aetheryte, currently not working due to the aetheryte actor not spawning
    327689,  -- Ul'dah: Steps of Nald Aetheryte, currently not working due to the aetheryte actor not spawning
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
    327813,  -- The Crystarium Aetheryte,currently not working due to the aetheryte actor not spawning
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
    327743,  -- The Gold Saucer: Entrance & Card Squares Aethernet shard
    327744,  -- The Gold Saucer: Wonder Square East Aethernet shard
    327745,  -- The Gold Saucer: Wonder Square West Aethernet shard
    327746,  -- The Gold Saucer: Event Square Aethernet shard
    327747,  -- The Gold Saucer: Cactpot Board Aethernet shard
    327748,  -- The Gold Saucer: Round Square Aethernet shard
    327749,  -- The Gold Saucer: Chocobo Square Aethernet shard
    -- Heavensward Aetherytes
    327760,  -- Ishgard: The Forgotten Knight Aethernet shard
    327761,  -- Ishgard: Skysteel Manufactory Aethernet shard
    327762,  -- Ishgard: The Brume Aethernet shard
    327763,  -- Ishgard: Anathaeum Astrologicum Aethernet shard
    327764,  -- Ishgard: The Jewled Crozier Aethernet shard
    327765,  -- Ishgard: Saint Reymanaud's Cathedral Aethernet shard
    327766,  -- Ishgard: The Tribunal Aethernet shard
    327767,  -- Ishgard: The Last Vigil Aethernet shard
    327769,  -- The Gold Saucer: Minion Square Aethernet shard
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
    [721044] = "CrystalBell.lua",
    [721098] = "HuntBoard.lua",
    [721226] = "Orchestrion.lua",
    [721347] = "GlamourDresser.lua",
    [721440] = "SummoningBell.lua",
    [720935] = "MarketBoard.lua",
    [720978] = "Armoire.lua",
    [1179657] = "Chocobokeep.lua", -- Chocobokeep in Central Shroud
}

-- Events in /common that aren't already covered by other tables
common_events = {
    [393227] = "GenericLevemete.lua",
    [720915] = "GenericMender.lua",
    [721480] = "GenericGemstoneTrader.lua", -- Generic Shadowbringers in-city gemstone traders
    [721479] = "GenericGemstoneTrader.lua", -- Generic Shadowbringers per-zone gemstone traders
    -- [721619] = "GenericGemstoneTrader.lua", -- Generic Endwalker & Dawntrail per-zone gemstone traders, but they do nothing when interacted with right now
    -- [721620] = "GenericGemstoneTrader.lua", -- Generic Endwalker & Dawntrail in-city gemstone traders, but they do nothing when interacted with right now
}

-- NPC shops that accept gil for purchasing items
generic_gil_shops = {
    262157, -- Tanie <Florist>, New Gridania
    262190, -- Blue Lily <Independent Apothecary>, Limsa Lominsa: The Lower Decks
    262197, -- Gerulf <Independent Culinarian>, Limsa Lominsa: The Lower Decks
    262574, -- Minon Trader, Chocobo Square
    262612, -- Tack & Feed Trader, Chocobo Square
    262735, -- Sorcha <Independent Jeweler> (Limsa Lominsa: The Lower Decks), Battlecraft Accessories
    262736, -- Sorcha <Independent Jeweler> (Limsa Lominsa: The Lower Decks), Fieldcraft/Tradecraft Accessories
    263220, -- Neon <Air-wheeler dealer>, Solution Nine
}

-- Not all Hunt NPCs are spawning right now, unfortunately.
generic_currency_exchange = {
    1769533, -- Gold Saucer Attendant <Prize Claim> (behind counter) -> Prize Exchange (Gear)
    1769544, -- Gold Saucer Attendant <Prize Claim> (behind counter) -> Prize Exchange (Weapons)
    1769545, -- Triple Triad Trader (Gold Saucer, behind counter) -> Prize Exchange (Weapons)
    1769546, -- Tack & Feed Trader (Gold Saucer, Chocobo Square) -> Race Items
    1769547, -- Tack & Feed Trader (Gold Saucer, Chocobo Square) -> Chocobo Training Manuals I
    1769548, -- Tack & Feed Trader (Gold Saucer, Chocobo Square) -> Chocobo Training Manuals II
    1769626, -- Minion Trader (Gold Saucer, Minion Square) -> Purchase Minions (MGP)
    1769637, -- Modern Aesthetics Saleswoman (Gold Saucer) -> Prize Exchange III
    1769660, -- Ishgard: Yolaine -> Doman Gear Exchange (DoW, IL 180)
    1769661, -- Ishgard: Yolaine -> Doman Gear Exchange (DoM, IL 180)
    1769715, -- Ishgard: Yolaine -> Artifact Gear Exchange I (DoW, IL 210)
    1769716, -- Ishgard: Yolaine -> Artifact Gear Exchange II (DoW, IL 210)
    1769717, -- Ishgard: Yolaine -> Artifact Gear Exchange (DoM, IL 210)
    1769751, -- Gold Saucer Attendant <Prize Claim> (by Modern Aestherics Saleswoman) -> Prize Exchange I
    1769752, -- Gold Saucer Attendant <Prize Claim> (by Modern Aestherics Saleswoman) -> Prize Exchange III
    1769783, -- Kugane: Satsuya -> Centurio Seal Exchange II
    1769864, -- Kugane: Satsuya -> Ala Mhigan Gear Exchange (DoW, IL 310)
    1769865, -- Kugane: Satsuya -> Ala Mhigan Gear Exchange (DoM, IL 310)
    1769914, -- Kugane: Satsuya -> Lost Allagan Gear (DoW, IL 340)
    1769915, -- Kugane: Satsuya -> Lost Allagan Gear (DoM, IL 340)
    1770476, -- Radz-at-Han: Wilmetta -> Sacks of Nuts Exchange
    1770538, -- Gold Saucer Attendant <Prize Claim> (by Modern Aestherics Saleswoman) -> Prize Exchange II
    1770599, -- Gold Saucer Attendant <Prize Claim> (behind counter) -> Prize Exchange (Registrable Miscellany)
    1770600, -- Gold Saucer Attendant <Prize Claim> (behind counter) -> Prize Exchange (Music/Furnishings)
    1770619, -- Radz-at-Han: Wilmetta -> Moonward Gear Exchange (DoW, IL 570)
    1770620, -- Radz-at-Han: Wilmetta -> Moonward Gear Exchange (DoW, IL 570)
    1770704, -- Radz-at-Han: Wilmetta -> Radiant's Gear (DoW, IL 600)
    1770705, -- Radz-at-Han: Wilmetta -> Radiant's Gear (DoM, IL 600)
    1770761, -- Tuliyollal: Ryubool Ja -> Dawn Hunt Vendor
    -- 3539075, -- Dibourdier <Mahjong Vendor> doesn't respond when interacted with right now, probably needs special handling
}

solution_nine_teleporters = {
    4194305, -- Teleporter from eastern Aetheryte Plaza to Recreation Zone
    4194306, -- Teleporter from Recreation Zone to eastern Aetheryte Plaza
    4194307, -- Teleporter from northern Aetheryte Plaza to Government Sector
    4194308, -- Teleporter from Government Sector to northern Aetheryte Plaza
    4194309, -- Teleporter from Nexus Arcade ground floor to upper balcony
    4194310, -- Teleporter from upper balcony to Nexus Arcade ground floor
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
TRIGGER_DIR = "events/walkin_trigger/"

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

for _, event_id in pairs(generic_gil_shops) do
    registerEvent(event_id, "events/common/GilShopkeeper.lua")
end

for _, event_id in pairs(generic_currency_exchange) do
    registerEvent(event_id, "events/common/GenericHuntCurrencyExchange.lua") --TODO: Should probably rename this since it now covers other generic currency vendors like Gold Saucer ones
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

for _, event_id in pairs(solution_nine_teleporters) do
    registerEvent(event_id, TRIGGER_DIR.."SolutionNineTeleporter.lua")
end
