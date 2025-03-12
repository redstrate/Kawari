use binrw::binrw;

use crate::common::{read_string, write_string};

// NOTE: See https://github.com/karashiiro/FFXIVOpcodes/blob/master/FFXIVOpcodes/Ipcs.cs for opcodes

#[binrw]
#[brw(repr = u16)]
#[derive(Clone, PartialEq, Debug)]
pub enum IPCOpCode {
    /// Sent by the server to Initialize something chat-related?
    InitializeChat = 0x2,
    /// Sent by the client when it requests the character list in the lobby.
    RequestCharacterList = 0x3,
    /// Sent by the client when it requests to enter a world.
    RequestEnterWorld = 0x4,
    /// Sent by the client after exchanging encryption information with the lobby server.
    ClientVersionInfo = 0x5,
    /// Sent by the client when they request something about the character (e.g. deletion.)
    LobbyCharacterAction = 0xB,
    /// Sent by the server to inform the client of their service accounts.
    LobbyServiceAccountList = 0xC,
    /// Sent by the server to inform the client of their characters.
    LobbyCharacterList = 0xD,
    /// Sent by the server to tell the client how to connect to the world server.
    LobbyEnterWorld = 0xF,
    /// Sent by the server to inform the client of their servers.
    LobbyServerList = 0x15,
    /// Sent by the server to inform the client of their retainers.
    LobbyRetainerList = 0x17,

    /// Sent by the client when they successfully initialize with the server, and they need several bits of information (e.g. what zone to load)
    InitRequest = 0x2ED,
    /// Sent by the server as response to ZoneInitRequest.
    InitResponse = 280, // TODO: probably wrong!
    /// Sent by the server that tells the client which zone to load
    InitZone = 0x0311,
    /// Sent by the server for... something
    ActorControlSelf = 0x018C,
    /// Sent by the server containing character stats
    PlayerStats = 0x01FA,
    /// Sent by the server to setup the player on the client
    PlayerSetup = 0x006B,
    // Sent by the server to setup class info
    UpdateClassInfo = 0x006A,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ServiceAccount {
    pub id: u32,
    pub unk1: u32,
    pub index: u32,
    #[bw(pad_size_to = 0x44)]
    #[br(count = 0x44)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct Server {
    pub id: u16,
    pub index: u16,
    pub flags: u32,
    #[brw(pad_before = 4)]
    #[brw(pad_after = 4)]
    pub icon: u32,
    #[bw(pad_size_to = 0x40)]
    #[br(count = 0x40)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct CharacterDetails {
    #[brw(pad_after = 4)]
    pub id: u32,
    pub content_id: u64,
    #[brw(pad_after = 4)]
    pub index: u32,
    pub origin_server_id: u16,
    pub current_server_id: u16,
    pub unk1: [u8; 16],
    #[bw(pad_size_to = 32)]
    #[br(count = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub character_name: String,
    #[bw(pad_size_to = 32)]
    #[br(count = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub origin_server_name: String,
    #[bw(pad_size_to = 32)]
    #[br(count = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub current_server_name: String,
    #[bw(pad_size_to = 1024)]
    #[br(count = 1024)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub character_detail_json: String,
    pub unk2: [u8; 20],
}

#[binrw]
#[brw(repr = u8)]
#[derive(Clone, PartialEq, Debug)]
pub enum LobbyCharacterAction {
    ReserveName = 0x1,
    Create = 0x2,
    Rename = 0x3,
    Delete = 0x4,
    Move = 0x5,
    RemakeRetainer = 0x6,
    RemakeChara = 0x7,
    SettingsUploadBegin = 0x8,
    SettingsUpload = 0xC,
    WorldVisit = 0xE,
    DataCenterToken = 0xF,
    Request = 0x15,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[binrw]
#[derive(Debug, Eq, PartialEq, Clone)]
#[brw(repr = u16)]
pub enum ActorControlType {
    SetCharaGearParamUI = 0x260,
}

#[binrw]
#[br(import(magic: &IPCOpCode))]
#[derive(Debug, Clone)]
pub enum IPCStructData {
    // Client->Server IPC
    #[br(pre_assert(*magic == IPCOpCode::ClientVersionInfo))]
    ClientVersionInfo {
        #[brw(pad_before = 18)] // full of nonsense i don't understand yet
        #[br(count = 64)]
        #[br(map = read_string)]
        #[bw(ignore)]
        session_id: String,

        #[brw(pad_before = 8)] // empty
        #[br(count = 128)]
        #[br(map = read_string)]
        #[bw(ignore)]
        version_info: String,
        // unknown stuff at the end, it's not completely empty'
    },
    #[br(pre_assert(*magic == IPCOpCode::RequestCharacterList))]
    RequestCharacterList {
        #[brw(pad_before = 16)]
        sequence: u64,
        // TODO: what is in here?
    },
    #[br(pre_assert(*magic == IPCOpCode::LobbyCharacterAction))]
    LobbyCharacterAction {
        request_number: u32,
        unk1: u32,
        character_id: u64,
        #[br(pad_before = 8)]
        character_index: u8,
        action: LobbyCharacterAction,
        world_id: u16,
        #[bw(pad_size_to = 32)]
        #[br(count = 32)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        name: String,
        // TODO: what else is in here?
        // according to TemporalStatis, chara make data? (probably op specific)
    },
    #[br(pre_assert(*magic == IPCOpCode::RequestEnterWorld))]
    RequestEnterWorld {
        #[brw(pad_before = 16)]
        sequence: u64,
        lookup_id: u64,
        // TODO: what else is in here?
    },
    #[br(pre_assert(*magic == IPCOpCode::InitRequest))]
    InitRequest {
        // TODO: full of possibly interesting information
        #[br(dbg)]
        unk: [u8; 105],
    },

    // Server->Client IPC
    #[br(pre_assert(false))]
    LobbyServiceAccountList {
        #[br(dbg)]
        sequence: u64,
        #[brw(pad_before = 1)]
        num_service_accounts: u8,
        unk1: u8,
        #[brw(pad_after = 4)]
        unk2: u8,
        #[br(count = 8)]
        service_accounts: Vec<ServiceAccount>,
    },
    #[br(pre_assert(false))]
    LobbyServerList {
        sequence: u64,
        unk1: u16,
        offset: u16,
        #[brw(pad_after = 8)]
        num_servers: u32,
        #[br(count = 6)]
        servers: Vec<Server>,
    },
    #[br(pre_assert(false))]
    LobbyRetainerList {
        // TODO: what is in here?
        #[brw(pad_before = 7)]
        #[brw(pad_after = 202)]
        unk1: u8,
    },
    #[br(pre_assert(false))]
    LobbyCharacterList {
        sequence: u64,
        counter: u8,
        #[brw(pad_after = 2)]
        num_in_packet: u8,
        unk1: u8,
        unk2: u8,
        unk3: u8,
        /// Set to 128 if legacy character
        unk4: u8,
        unk5: [u32; 7],
        unk6: u8,
        veteran_rank: u8,
        #[brw(pad_after = 1)]
        unk7: u8,
        days_subscribed: u32,
        remaining_days: u32,
        days_to_next_rank: u32,
        max_characters_on_world: u16,
        unk8: u16,
        #[brw(pad_after = 12)]
        entitled_expansion: u32,
        #[br(count = 2)]
        characters: Vec<CharacterDetails>,
    },
    #[br(pre_assert(false))]
    LobbyEnterWorld {
        sequence: u64,
        character_id: u32,
        #[brw(pad_before = 4)]
        content_id: u64,
        #[brw(pad_before = 4)]
        #[bw(pad_size_to = 66)]
        #[br(count = 66)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        session_id: String,
        port: u16,
        #[brw(pad_after = 16)]
        #[br(count = 48)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        host: String,
    },
    #[br(pre_assert(false))]
    InitializeChat { unk: [u8; 24] },
    #[br(pre_assert(false))]
    InitResponse {
        unk1: u64,
        character_id: u32,
        unk2: u32,
    },
    #[br(pre_assert(false))]
    InitZone {
        server_id: u16,
        zone_id: u16,
        zone_index: u16,
        content_finder_condition_id: u16,
        layer_set_id: u32,
        layout_id: u32,
        weather_id: u32,
        unk_bitmask1: u8,
        unk_bitmask2: u8,
        unk1: u8,
        unk2: u32,
        festival_id: u16,
        additional_festival_id: u16,
        unk3: u32,
        unk4: u32,
        unk5: u32,
        unk6: [u32; 4],
        unk7: [u32; 3],
        position: Position,
        unk8: [u32; 4],
        unk9: u32,
    },
    #[br(pre_assert(false))]
    ActorControlSelf {
        #[brw(pad_after = 2)]
        category: ActorControlType,
        param1: u32,
        param2: u32,
        param3: u32,
        param4: u32,
        param5: u32,
        #[brw(pad_after = 4)]
        param6: u32,
    },
    #[br(pre_assert(false))]
    PlayerStats {
        strength: u32,
        dexterity: u32,
        vitality: u32,
        intelligence: u32,
        mind: u32,
        piety: u32,
        hp: u32,
        mp: u32,
        tp: u32,
        gp: u32,
        cp: u32,
        delay: u32,
        tenacity: u32,
        attack_power: u32,
        defense: u32,
        direct_hit_rate: u32,
        evasion: u32,
        magic_defense: u32,
        critical_hit: u32,
        attack_magic_potency: u32,
        healing_magic_potency: u32,
        elemental_bonus: u32,
        determination: u32,
        skill_speed: u32,
        spell_speed: u32,
        haste: u32,
        craftmanship: u32,
        control: u32,
        gathering: u32,
        perception: u32,
        unk1: [u32; 26],
    },
    #[br(pre_assert(false))]
    PlayerSetup {
        content_id: u64,
        crest: u64,
        unknown10: u64,
        char_id: u32,
        rested_exp: u32,
        companion_current_exp: u32,
        unknown1c: u32,
        fish_caught: u32,
        use_bait_catalog_id: u32,
        unknown28: u32,
        unknown_pvp2c: u16,
        unknown2e: u16,
        pvp_frontline_overall_campaigns: u32,
        unknown_timestamp34: u32,
        unknown_timestamp38: u32,
        unknown3c: u32,
        unknown40: u32,
        unknown44: u32,
        companion_time_passed: f32,
        unknown4c: u32,
        unknown50: u16,
        unknown_pvp52: [u16; 4],
        pvp_series_exp: u16,
        player_commendations: u16,
        unknown64: [u16; 8],
        pvp_rival_wings_total_matches: u16,
        pvp_rival_wings_total_victories: u16,
        pvp_rival_wings_weekly_matches: u16,
        pvp_rival_wings_weekly_victories: u16,
        max_level: u8,
        expansion: u8,
        unknown76: u8,
        unknown77: u8,
        unknown78: u8,
        race: u8,
        tribe: u8,
        gender: u8,
        current_job: u8,
        current_class: u8,
        deity: u8,
        nameday_month: u8,
        nameday_day: u8,
        city_state: u8,
        homepoint: u8,
        unknown8d: [u8; 3],
        companion_rank: u8,
        companion_stars: u8,
        companion_sp: u8,
        companion_unk93: u8,
        companion_color: u8,
        companion_fav_feed: u8,
        fav_aetheryte_count: u8,
        unknown97: [u8; 5],
        sightseeing21_to_80_unlock: u8,
        sightseeing_heavensward_unlock: u8,
        unknown9e: [u8; 26],
        exp: [u32; 32],
        pvp_total_exp: u32,
        unknown_pvp124: u32,
        pvp_exp: u32,
        pvp_frontline_overall_ranks: [u32; 3],
        unknown138: u32,
        levels: [u16; 32],
        unknown194: [u8; 218],
        companion_name: [u8; 21],
        companion_def_rank: u8,
        companion_att_rank: u8,
        companion_heal_rank: u8,
        mount_guide_mask: [u8; 33],
        ornament_mask: [u8; 4],
        unknown281: [u8; 23],
        #[br(count = 32)]
        #[bw(pad_size_to = 32)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        name: String,
        unknown293: [u8; 16],
        unknown2a3: u8,
        unlock_bitmask: [u8; 64],
        aetheryte: [u8; 26],
        favorite_aetheryte_ids: [u16; 4],
        free_aetheryte_id: u16,
        ps_plus_free_aetheryte_id: u16,
        discovery: [u8; 480],
        howto: [u8; 36],
        unknown554: [u8; 4],
        minions: [u8; 60],
        chocobo_taxi_mask: [u8; 12],
        watched_cutscenes: [u8; 159],
        companion_barding_mask: [u8; 12],
        companion_equipped_head: u8,
        companion_equipped_body: u8,
        companion_equipped_legs: u8,
        unknown_mask: [u8; 287],
        pose: [u8; 7],
        unknown6df: [u8; 3],
        challenge_log_complete: [u8; 13],
        secret_recipe_book_mask: [u8; 12],
        unknown_mask6f7: [u8; 29],
        relic_completion: [u8; 12],
        sightseeing_mask: [u8; 37],
        hunting_mark_mask: [u8; 102],
        triple_triad_cards: [u8; 45],
        unknown895: u8,
        unknown7d7: [u8; 15],
        unknown7d8: u8,
        unknown7e6: [u8; 49],
        regional_folklore_mask: [u8; 6],
        orchestrion_mask: [u8; 87],
        hall_of_novice_completion: [u8; 3],
        anima_completion: [u8; 11],
        unknown85e: [u8; 41],
        unlocked_raids: [u8; 28],
        unlocked_dungeons: [u8; 18],
        unlocked_guildhests: [u8; 10],
        unlocked_trials: [u8; 12],
        unlocked_pvp: [u8; 5],
        cleared_raids: [u8; 28],
        cleared_dungeons: [u8; 18],
        cleared_guildhests: [u8; 10],
        cleared_trials: [u8; 12],
        cleared_pvp: [u8; 5],
        unknown948: [u8; 15],
    },
    #[br(pre_assert(false))]
    UpdateClassInfo {
        class_id: u16,
        unknown: u8,
        is_specialist: u8,
        synced_level: u16,
        class_level: u16,
        role_actions: [u32; 10],
    },
}

#[binrw]
#[derive(Debug, Clone)]
pub struct IPCSegment {
    pub unk1: u8,
    pub unk2: u8,
    #[br(dbg)]
    pub op_code: IPCOpCode,
    #[brw(pad_before = 2)] // empty
    #[br(dbg)]
    pub server_id: u16,
    #[br(dbg)]
    pub timestamp: u32,
    #[brw(pad_before = 4)]
    #[br(args(&op_code))]
    pub data: IPCStructData,
}

impl IPCSegment {
    pub fn calc_size(&self) -> u32 {
        let header = 16;
        header
            + match self.data {
                IPCStructData::ClientVersionInfo { .. } => todo!(),
                IPCStructData::LobbyServiceAccountList { .. } => 24 + (8 * 80),
                IPCStructData::RequestCharacterList { .. } => todo!(),
                IPCStructData::LobbyServerList { .. } => 24 + (6 * 84),
                IPCStructData::LobbyRetainerList { .. } => 210,
                IPCStructData::LobbyCharacterList { .. } => 80 + (2 * 1184),
                IPCStructData::LobbyCharacterAction { .. } => todo!(),
                IPCStructData::LobbyEnterWorld { .. } => 160,
                IPCStructData::RequestEnterWorld { .. } => todo!(),
                IPCStructData::InitializeChat { .. } => 24,
                IPCStructData::InitRequest { .. } => todo!(),
                IPCStructData::InitResponse { .. } => 16,
                IPCStructData::InitZone { .. } => 103,
                IPCStructData::ActorControlSelf { .. } => 32,
                IPCStructData::PlayerStats { .. } => 228,
                IPCStructData::PlayerSetup { .. } => 2544,
                IPCStructData::UpdateClassInfo { .. } => 48,
            }
    }
}
