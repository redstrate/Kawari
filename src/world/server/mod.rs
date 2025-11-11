use binrw::{BinRead, BinWrite};
use std::{
    collections::{HashMap, VecDeque},
    env::consts::EXE_SUFFIX,
    io::Cursor,
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::mpsc::Receiver;

use crate::{
    common::{
        CustomizeData, GameData, INVALID_OBJECT_ID, JumpState, MoveAnimationState,
        MoveAnimationType, ObjectId, ObjectTypeId, ObjectTypeKind, Position, euler_to_direction,
    },
    config::get_config,
    ipc::{
        chat::{PartyMessage, TellNotFoundError},
        zone::{
            ActorControl, ActorControlCategory, ActorControlSelf, ActorControlTarget,
            BattleNpcSubKind, ClientTriggerCommand, CommonSpawn, Conditions, NpcSpawn, ObjectKind,
            OnlineStatus, OnlineStatusMask, PartyMemberEntry, PartyUpdateStatus, PlayerEntry,
            PlayerSpawn, ServerZoneIpcData, ServerZoneIpcSegment, SocialListRequestType,
            SocialListUIFlags,
        },
    },
    opcodes::ServerZoneIpcType,
    packet::{IpcSegmentHeader, PacketSegment, SegmentData, SegmentType, ServerIpcSegmentHeader},
    world::{MessageInfo, Navmesh, common::PartyUpdateTargets, server::zone::Zone},
};

use super::{Actor, ClientHandle, ClientId, FromServer, ToServer};

use crate::world::common::SpawnKind;

mod zone;

/// Used for the debug NPC.
pub const CUSTOMIZE_DATA: CustomizeData = CustomizeData {
    race: 4,
    gender: 1,
    age: 1,
    height: 50,
    subrace: 7,
    face: 3,
    hair: 5,
    enable_highlights: 0,
    skin_tone: 10,
    right_eye_color: 75,
    hair_tone: 50,
    highlights: 0,
    facial_features: 1,
    facial_feature_color: 19,
    eyebrows: 1,
    left_eye_color: 75,
    eyes: 1,
    nose: 0,
    jaw: 1,
    mouth: 1,
    lips_tone_fur_pattern: 169,
    race_feature_size: 100,
    race_feature_type: 1,
    bust: 100,
    face_paint: 0,
    face_paint_color: 167,
};

#[derive(Debug, Clone)]
enum NetworkedActor {
    Player(PlayerSpawn),
    Npc {
        current_path: VecDeque<[f32; 3]>,
        current_path_lerp: f32,
        current_target: Option<ObjectId>,
        last_position: Option<Position>,
        spawn: NpcSpawn,
    },
}

impl NetworkedActor {
    pub fn get_common_spawn(&self) -> &CommonSpawn {
        match &self {
            NetworkedActor::Player(player_spawn) => &player_spawn.common,
            NetworkedActor::Npc { spawn, .. } => &spawn.common,
        }
    }

    pub fn get_player_spawn(&self) -> Option<&PlayerSpawn> {
        match &self {
            NetworkedActor::Player(player_spawn) => Some(player_spawn),
            NetworkedActor::Npc { .. } => None,
        }
    }
}

#[derive(Default, Debug)]
enum NavmeshGenerationStep {
    /// No generation is currently happening.
    #[default]
    None,
    /// We need to generate a navmesh at this path.
    Needed(String),
    /// The process to write the navmesh has started, and we need to wait until the file exists.
    Started(String),
}

// TODO: structure is temporary, of course
#[derive(Default, Debug)]
struct Instance {
    actors: HashMap<ObjectId, NetworkedActor>,
    navmesh: Navmesh,
    zone: Zone,
    weather_id: u16,
    /// If Some, then this is the path of the navmesh we need to generate.
    generate_navmesh: NavmeshGenerationStep,
}

impl Instance {
    pub fn new(id: u16, game_data: &mut GameData) -> Self {
        let mut instance = Instance {
            zone: Zone::load(game_data, id),
            weather_id: game_data.get_weather(id as u32).unwrap_or_default() as u16,
            ..Default::default()
        };

        let config = get_config();
        if config.filesystem.navimesh_path.is_empty() {
            tracing::warn!("Navimesh path is not set! Monsters will not function correctly!");
        } else {
            let mut nvm_path = PathBuf::from(config.filesystem.navimesh_path);
            nvm_path.push(instance.zone.navimesh_path.clone());

            if let Ok(nvm_bytes) = std::fs::read(&nvm_path) {
                if let Some(navmesh) = Navmesh::from_existing(&nvm_bytes) {
                    instance.navmesh = navmesh;

                    tracing::info!("Successfully loaded navimesh from {nvm_path:?}");
                } else {
                    tracing::warn!(
                        "Failed to read {nvm_path:?}, monsters will not function correctly!"
                    );
                }
            } else if config.world.generate_navmesh {
                instance.generate_navmesh =
                    NavmeshGenerationStep::Needed(nvm_path.to_str().unwrap().to_string());
            } else {
                tracing::warn!(
                    "Failed to read {nvm_path:?}, monsters will not function correctly!"
                );
            }
        }

        instance
    }

    fn find_actor(&self, id: ObjectId) -> Option<&NetworkedActor> {
        self.actors.get(&id)
    }

    fn find_actor_mut(&mut self, id: ObjectId) -> Option<&mut NetworkedActor> {
        self.actors.get_mut(&id)
    }

    fn insert_npc(&mut self, id: ObjectId, spawn: NpcSpawn) {
        self.actors.insert(
            id,
            NetworkedActor::Npc {
                current_path: VecDeque::default(),
                current_path_lerp: 0.0,
                current_target: None,
                last_position: None,
                spawn,
            },
        );
    }

    fn generate_actor_id() -> u32 {
        // TODO: ensure we don't collide with another actor
        fastrand::u32(..)
    }

    fn find_all_players(&self) -> Vec<ObjectId> {
        self.actors
            .iter()
            .filter(|(_, y)| matches!(y, NetworkedActor::Player(_)))
            .map(|(x, _)| *x)
            .collect()
    }
}

#[derive(Default, Debug, Clone)]
struct ClientState {}

#[derive(Default, Debug)]
struct WorldServer {
    /// Indexed by zone id
    instances: HashMap<u16, Instance>,
}

impl WorldServer {
    /// Ensures an instance exists, and creates one if not found.
    fn ensure_exists(&mut self, zone_id: u16, game_data: &mut GameData) {
        // create a new instance if necessary
        self.instances
            .entry(zone_id)
            .or_insert_with(|| Instance::new(zone_id, game_data));
    }

    /// Finds the instance associated with a zone, or None if it doesn't exist yet.
    fn find_instance(&self, zone_id: u16) -> Option<&Instance> {
        self.instances.get(&zone_id)
    }

    /// Finds the instance associated with a zone, or creates it if it doesn't exist yet
    fn find_instance_mut(&mut self, zone_id: u16) -> &mut Instance {
        self.instances.entry(zone_id).or_default()
    }

    /// Finds the instance associated with an actor, or returns None if they are not found.
    fn find_actor_instance(&self, actor_id: u32) -> Option<&Instance> {
        self.instances
            .values()
            .find(|instance| instance.actors.contains_key(&ObjectId(actor_id)))
    }

    /// Finds the instance associated with an actor, or returns None if they are not found.
    fn find_actor_instance_mut(&mut self, actor_id: u32) -> Option<&mut Instance> {
        self.instances
            .values_mut()
            .find(|instance| instance.actors.contains_key(&ObjectId(actor_id)))
    }
}

// TODO: Probably move party-related structs into their own file?
#[derive(Clone, Debug)]
struct PartyMember {
    pub actor_id: ObjectId,
    pub zone_client_id: ClientId,
    pub chat_client_id: ClientId,
    pub content_id: u64,
    pub account_id: u64,
    pub world_id: u16,
    pub name: String,
}

impl Default for PartyMember {
    fn default() -> Self {
        Self {
            actor_id: INVALID_OBJECT_ID,
            zone_client_id: ClientId::default(),
            chat_client_id: ClientId::default(),
            content_id: 0,
            account_id: 0,
            world_id: 0,
            name: String::default(),
        }
    }
}

impl PartyMember {
    pub fn is_valid(&self) -> bool {
        self.actor_id != INVALID_OBJECT_ID
    }

    pub fn is_online(&self) -> bool {
        self.zone_client_id != ClientId::default() && self.chat_client_id != ClientId::default()
    }
}

#[derive(Clone, Debug, Default)]
struct Party {
    members: [PartyMember; PartyMemberEntry::NUM_ENTRIES],
    leader_id: u32,
    chatchannel_id: u32, // There's no reason to store a full u64/ChatChannel here, as it's created properly in the chat connection!
}

impl Party {
    pub fn get_member_count(&self) -> usize {
        self.members.iter().filter(|x| x.is_valid()).count()
    }

    pub fn get_online_member_count(&self) -> usize {
        self.members
            .iter()
            .filter(|x| x.is_valid() && x.is_online())
            .count()
    }

    pub fn remove_member(&mut self, member_to_remove: u32) {
        for member in self.members.iter_mut() {
            if member.actor_id.0 == member_to_remove {
                *member = PartyMember::default();
                break;
            }
        }
    }

    pub fn set_member_offline(&mut self, offline_member: u32) {
        for member in self.members.iter_mut() {
            if member.actor_id.0 == offline_member {
                member.zone_client_id = ClientId::default();
                member.chat_client_id = ClientId::default();
                break;
            }
        }
    }

    pub fn auto_promote_member(&mut self) -> u32 {
        for member in &self.members {
            if member.is_valid() && member.is_online() && member.actor_id.0 != self.leader_id {
                self.leader_id = member.actor_id.0;
                break;
            }
        }

        self.leader_id
    }

    pub fn get_member_by_content_id(&self, content_id: u64) -> Option<PartyMember> {
        for member in &self.members {
            if member.content_id == content_id {
                return Some(member.clone());
            }
        }
        None
    }
    pub fn get_member_by_actor_id(&self, actor_id: u32) -> Option<PartyMember> {
        for member in &self.members {
            if member.actor_id.0 == actor_id {
                return Some(member.clone());
            }
        }
        None
    }
}

#[derive(Default, Debug)]
struct NetworkState {
    to_remove: Vec<ClientId>,
    to_remove_chat: Vec<ClientId>,
    clients: HashMap<ClientId, (ClientHandle, ClientState)>,
    chat_clients: HashMap<ClientId, (ClientHandle, ClientState)>,
    parties: HashMap<u64, Party>,
}

#[derive(Debug, PartialEq)]
enum DestinationNetwork {
    ZoneClients,
    ChatClients,
}

impl NetworkState {
    /// Tell all the clients that a new actor spawned.
    fn send_actor(&mut self, actor: Actor, spawn: SpawnKind) {
        // TODO: only send in the relevant instance
        for (id, (handle, _)) in &mut self.clients {
            let id = *id;

            let msg = FromServer::ActorSpawn(actor, spawn.clone());

            if handle.send(msg).is_err() {
                self.to_remove.push(id);
            }
        }
    }

    /// Inform all clients in an instance that the actor has left.
    fn inform_remove_actor(&mut self, instance: &Instance, from_id: ClientId, actor_id: u32) {
        for (id, (handle, _)) in &mut self.clients {
            let id = *id;

            // Don't bother telling the client who told us
            if id == from_id {
                continue;
            }

            // Skip any clients not in this instance
            if !instance.actors.contains_key(&ObjectId(handle.actor_id)) {
                continue;
            }

            let msg = FromServer::ActorDespawn(actor_id);

            if handle.send(msg).is_err() {
                self.to_remove.push(id);
            }
        }
    }

    fn send_to_all(
        &mut self,
        id_to_skip: Option<ClientId>,
        message: FromServer,
        destination: DestinationNetwork,
    ) {
        let clients = match destination {
            DestinationNetwork::ZoneClients => &mut self.clients,
            DestinationNetwork::ChatClients => &mut self.chat_clients,
        };

        for (id, (handle, _)) in clients {
            let id = *id;
            if let Some(id_to_skip) = id_to_skip
                && id == id_to_skip
            {
                continue;
            }

            if handle.send(message.clone()).is_err() {
                if destination == DestinationNetwork::ZoneClients {
                    self.to_remove.push(id);
                } else {
                    self.to_remove_chat.push(id);
                }
            }
        }
    }

    fn send_to(
        &mut self,
        client_id: ClientId,
        message: FromServer,
        destination: DestinationNetwork,
    ) {
        let clients = match destination {
            DestinationNetwork::ZoneClients => &mut self.clients,
            DestinationNetwork::ChatClients => &mut self.chat_clients,
        };

        for (id, (handle, _)) in clients {
            let id = *id;

            if id == client_id {
                if handle.send(message).is_err() {
                    if destination == DestinationNetwork::ZoneClients {
                        self.to_remove.push(id);
                    } else {
                        self.to_remove_chat.push(id);
                    }
                }
                break;
            }
        }
    }

    fn send_to_by_actor_id(
        &mut self,
        actor_id: u32,
        message: FromServer,
        destination: DestinationNetwork,
    ) {
        let clients = match destination {
            DestinationNetwork::ZoneClients => &mut self.clients,
            DestinationNetwork::ChatClients => &mut self.chat_clients,
        };

        for (id, (handle, _)) in clients {
            let id = *id;

            if handle.actor_id == actor_id {
                if handle.send(message).is_err() {
                    if destination == DestinationNetwork::ZoneClients {
                        self.to_remove.push(id);
                    } else {
                        self.to_remove_chat.push(id);
                    }
                }
                break;
            }
        }
    }

    fn send_to_party(
        &mut self,
        party_id: u64,
        from_actor_id: Option<u32>,
        message: FromServer,
        destination: DestinationNetwork,
    ) {
        let Some(party) = self.parties.get(&party_id) else {
            return;
        };

        for member in &party.members {
            // Skip offline or otherwise non-existent members
            if member.actor_id == INVALID_OBJECT_ID || member.zone_client_id == ClientId::default()
            {
                continue;
            }

            // Skip a desired party member if needed.
            if let Some(from_actor_id) = from_actor_id
                && from_actor_id == member.actor_id.0
            {
                continue;
            }

            match destination {
                DestinationNetwork::ZoneClients => {
                    let handle = &mut self.clients.get_mut(&member.zone_client_id).unwrap().0;
                    if handle.send(message.clone()).is_err() {
                        self.to_remove.push(member.zone_client_id);
                    }
                }
                DestinationNetwork::ChatClients => {
                    let handle = &mut self.chat_clients.get_mut(&member.chat_client_id).unwrap().0;
                    if handle.send(message.clone()).is_err() {
                        self.to_remove_chat.push(member.chat_client_id);
                    }
                }
            }
        }
    }
}

fn build_party_list(party: &Party, data: &WorldServer) -> Vec<PartyMemberEntry> {
    let mut party_list = Vec::<PartyMemberEntry>::new();

    // Online members
    for instance in data.instances.values() {
        for (id, actor) in &instance.actors {
            let spawn = match actor {
                NetworkedActor::Player(spawn) => spawn,
                _ => continue,
            };
            for member in &party.members {
                if member.actor_id == *id {
                    party_list.push(PartyMemberEntry {
                        account_id: spawn.account_id,
                        content_id: spawn.content_id,
                        name: spawn.common.name.clone(),
                        actor_id: *id,
                        classjob_id: spawn.common.class_job,
                        classjob_level: spawn.common.level,
                        current_hp: spawn.common.hp_curr,
                        max_hp: spawn.common.hp_max,
                        current_mp: spawn.common.mp_curr,
                        max_mp: spawn.common.mp_max,
                        current_zone_id: instance.zone.id,
                        home_world_id: spawn.home_world_id,
                        ..Default::default()
                    });
                    break;
                }
            }
        }
    }

    // Offline members
    for member in &party.members {
        if member.is_valid() && !member.is_online() {
            party_list.push(PartyMemberEntry {
                account_id: member.account_id,
                content_id: member.content_id,
                name: member.name.clone(),
                home_world_id: member.world_id,
                actor_id: ObjectId(0), // It doesn't seem to matter, but retail sets offline members' actor ids to 0.
                ..Default::default()
            })
        }
    }

    party_list
}

fn do_change_zone(
    data: &mut WorldServer,
    network: &mut NetworkState,
    game_data: &mut GameData,
    destination_zone_id: u16,
    destination_instance_id: u32,
    actor_id: u32,
    from_id: ClientId,
) {
    // inform the players in this zone that this actor left
    if let Some(current_instance) = data.find_actor_instance_mut(actor_id) {
        current_instance.actors.remove(&ObjectId(actor_id));
        network.inform_remove_actor(current_instance, from_id, actor_id);
    }

    // then find or create a new instance with the zone id
    data.ensure_exists(destination_zone_id, game_data);
    let target_instance = data.find_instance_mut(destination_zone_id);

    let exit_position;
    let exit_rotation;
    if let Some((destination_object, _)) =
        target_instance.zone.find_pop_range(destination_instance_id)
    {
        exit_position = Position {
            x: destination_object.transform.translation[0],
            y: destination_object.transform.translation[1],
            z: destination_object.transform.translation[2],
        };
        exit_rotation = euler_to_direction(destination_object.transform.rotation);
    } else {
        exit_position = Position::default();
        exit_rotation = 0.0;
    }

    // now that we have all of the data needed, inform the connection of where they need to be
    let msg = FromServer::ChangeZone(
        destination_zone_id,
        target_instance.weather_id,
        exit_position,
        exit_rotation,
        target_instance.zone.to_lua_zone(target_instance.weather_id),
        false,
    );
    network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
}

fn set_player_minion(
    data: &mut WorldServer,
    network: &mut NetworkState,
    to_remove: &mut Vec<ClientId>,
    minion_id: u32,
    from_id: ClientId,
    from_actor_id: u32,
) {
    for (id, (handle, _)) in &mut network.clients {
        let id = *id;

        // Update our common spawn to reflect the new minion
        if id == from_id {
            let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                break;
            };

            let Some(actor) = instance.find_actor_mut(ObjectId(from_actor_id)) else {
                break;
            };

            let NetworkedActor::Player(player) = actor else {
                break;
            };

            player.common.active_minion = minion_id as u16;

            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                category: ActorControlCategory::MinionSpawnControl { minion_id },
            });

            if handle.send(msg).is_err() {
                to_remove.push(id);
            }

            // Skip sending the regular ActorControl to ourselves
            continue;
        }

        let msg = FromServer::ActorControl(
            from_actor_id,
            ActorControl {
                category: ActorControlCategory::MinionSpawnControl { minion_id },
            },
        );

        if handle.send(msg).is_err() {
            to_remove.push(id);
        }
    }
}

fn server_logic_tick(data: &mut WorldServer, network: &mut NetworkState) {
    for instance in data.instances.values_mut() {
        let mut actor_moves = Vec::new();
        let players = instance.find_all_players();

        let mut target_actor_pos = HashMap::new();

        // const pass
        for (id, actor) in &instance.actors {
            if let NetworkedActor::Npc {
                current_path,
                current_path_lerp,
                current_target,
                spawn,
                last_position,
            } = actor
                && current_target.is_some()
            {
                let needs_repath = current_path.is_empty();
                if !needs_repath {
                    // follow current path
                    let next_position = Position {
                        x: current_path[0][0],
                        y: current_path[0][1],
                        z: current_path[0][2],
                    };
                    let current_position = last_position.unwrap_or(spawn.common.pos);

                    let dir_x = current_position.x - next_position.x;
                    let dir_z = current_position.z - next_position.z;
                    let rotation = f32::atan2(-dir_z, dir_x).to_degrees();

                    actor_moves.push(FromServer::ActorMove(
                        id.0,
                        Position::lerp(current_position, next_position, *current_path_lerp),
                        rotation,
                        MoveAnimationType::RUNNING,
                        MoveAnimationState::None,
                        JumpState::NoneOrFalling,
                    ));
                }

                target_actor_pos.insert(
                    current_target.unwrap(),
                    instance
                        .find_actor(current_target.unwrap())
                        .unwrap()
                        .get_common_spawn()
                        .pos,
                );
            }
        }

        // mut pass
        for (id, actor) in &mut instance.actors {
            if let NetworkedActor::Npc {
                current_path,
                current_path_lerp,
                current_target,
                spawn,
                last_position,
            } = actor
            {
                // switch to the next node if we passed this one
                if *current_path_lerp >= 1.0 {
                    *current_path_lerp = 0.0;
                    if !current_path.is_empty() {
                        *last_position = Some(Position {
                            x: current_path[0][0],
                            y: current_path[0][1],
                            z: current_path[0][2],
                        });
                        current_path.pop_front();
                    }
                }

                if current_target.is_none() {
                    // find a player
                    if !players.is_empty() {
                        *current_target = Some(players[0]);
                    }
                } else if !current_path.is_empty() {
                    let next_position = Position {
                        x: current_path[0][0],
                        y: current_path[0][1],
                        z: current_path[0][2],
                    };
                    let current_position = last_position.unwrap_or(spawn.common.pos);
                    let distance = Position::distance(current_position, next_position);

                    // TODO: this doesn't work like it should
                    *current_path_lerp += (10.0 / distance).clamp(0.0, 1.0);
                }

                if target_actor_pos.contains_key(&current_target.unwrap()) {
                    let target_pos = target_actor_pos[&current_target.unwrap()];
                    let distance = Position::distance(spawn.common.pos, target_pos);
                    let needs_repath = current_path.is_empty() && distance > 5.0; // TODO: confirm distance this in retail
                    if needs_repath && current_target.is_some() {
                        let current_pos = spawn.common.pos;
                        *current_path = instance
                            .navmesh
                            .calculate_path(
                                [current_pos.x, current_pos.y, current_pos.z],
                                [target_pos.x, target_pos.y, target_pos.z],
                            )
                            .into();
                    }
                }

                // update common spawn
                for msg in &actor_moves {
                    if let FromServer::ActorMove(
                        msg_id,
                        pos,
                        rotation,
                        MoveAnimationType::RUNNING,
                        MoveAnimationState::None,
                        JumpState::NoneOrFalling,
                    ) = msg
                        && id.0 == *msg_id
                    {
                        spawn.common.pos = *pos;
                        spawn.common.rotation = *rotation;
                    }
                }
            }
        }

        // inform clients of the NPCs new positions
        for msg in actor_moves {
            for (handle, _) in network.clients.values_mut() {
                if handle.send(msg.clone()).is_err() {
                    //to_remove.push(id);
                }
            }
        }

        // generate navmesh if necessary
        match &instance.generate_navmesh {
            NavmeshGenerationStep::None => {}
            NavmeshGenerationStep::Needed(nvm_path) => {
                tracing::info!(
                    "Missing navmesh {nvm_path:?}, we are going to generate it in the background now..."
                );

                let mut dir = std::env::current_exe().unwrap();
                dir.pop();
                dir.push(format!("kawari-navimesh{EXE_SUFFIX}"));

                // start navimesh generator
                Command::new(dir)
                    .arg(instance.zone.id.to_string())
                    .arg(nvm_path)
                    .spawn()
                    .unwrap();

                instance.generate_navmesh = NavmeshGenerationStep::Started(nvm_path.clone());
            }
            NavmeshGenerationStep::Started(nvm_path) => {
                if let Ok(nvm_bytes) = std::fs::read(nvm_path) {
                    if let Some(navmesh) = Navmesh::from_existing(&nvm_bytes) {
                        instance.navmesh = navmesh;

                        tracing::info!("Successfully loaded navimesh from {nvm_path:?}");
                    } else {
                        tracing::warn!(
                            "Failed to read {nvm_path:?}, monsters will not function correctly!"
                        );
                    }
                    instance.generate_navmesh = NavmeshGenerationStep::None;
                }
            }
        }
    }
}

pub async fn server_main_loop(mut recv: Receiver<ToServer>) -> Result<(), std::io::Error> {
    let data = Arc::new(Mutex::new(WorldServer::default()));
    let network = Arc::new(Mutex::new(NetworkState::default()));
    let game_data = Arc::new(Mutex::new(GameData::new()));

    {
        let data = data.clone();
        let network = network.clone();
        tokio::task::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            interval.tick().await;
            loop {
                interval.tick().await;
                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();
                server_logic_tick(&mut data, &mut network);
            }
        });
    }

    while let Some(msg) = recv.recv().await {
        let mut to_remove = Vec::new();

        match msg {
            ToServer::NewClient(handle) => {
                tracing::info!(
                    "New zone client {:?} is connecting with actor id {}",
                    handle.id,
                    handle.actor_id
                );

                let mut network = network.lock().unwrap();
                let mut party_id = None;
                let handle_id = handle.id;

                // Refresh the party member's client id, if applicable.
                'outer: for (id, party) in &mut network.parties {
                    for member in &mut party.members {
                        if member.actor_id.0 == handle.actor_id {
                            member.zone_client_id = handle.id;
                            party_id = Some(*id);
                            break 'outer;
                        }
                    }
                }

                network
                    .clients
                    .insert(handle.id, (handle.clone(), ClientState::default()));

                if let Some(party_id) = party_id {
                    tracing::info!("{} is rejoining party {}", handle.actor_id, party_id);
                    network.send_to(
                        handle_id,
                        FromServer::RejoinPartyAfterDisconnect(party_id),
                        DestinationNetwork::ZoneClients,
                    );
                } else {
                    tracing::info!("{} was not in a party before connecting.", handle.actor_id);
                }
            }
            ToServer::NewChatClient(handle) => {
                tracing::info!(
                    "New chat client {:?} is connecting with actor id {}",
                    handle.id,
                    handle.actor_id
                );

                let mut network = network.lock().unwrap();

                // Refresh the party member's client id, if applicable.
                'outer: for party in &mut network.parties.values_mut() {
                    for member in &mut party.members {
                        if member.actor_id.0 == handle.actor_id {
                            member.chat_client_id = handle.id; // The chat connection doesn't get informed here since it'll happen later.
                            break 'outer;
                        }
                    }
                }

                network
                    .chat_clients
                    .insert(handle.id, (handle, ClientState::default()));
            }
            ToServer::ReadySpawnPlayer(from_id, zone_id, position, rotation) => {
                let mut network = network.lock().unwrap();
                let mut data = data.lock().unwrap();

                // create a new instance if necessary
                let mut game_data = game_data.lock().unwrap();
                data.ensure_exists(zone_id, &mut game_data);
                let target_instance = data.find_instance_mut(zone_id);

                // tell the client to load into the zone
                let msg = FromServer::ChangeZone(
                    zone_id,
                    target_instance.weather_id,
                    position,
                    rotation,
                    target_instance.zone.to_lua_zone(target_instance.weather_id),
                    true, // since this is initial login
                );

                network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
            }
            ToServer::ZoneLoaded(from_id, zone_id, player_spawn) => {
                tracing::info!(
                    "Client {from_id:?} has now loaded, sending them existing player data."
                );

                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();

                // Send existing player data to the connection, if any
                if let Some(instance) = data.find_instance(zone_id) {
                    // send existing player data
                    for (id, spawn) in &instance.actors {
                        let kind = match spawn {
                            NetworkedActor::Player(spawn) => SpawnKind::Player(spawn.clone()),
                            NetworkedActor::Npc { spawn, .. } => {
                                // TODO: Do we actually care about NPCs here if we're only sending *player* data?
                                SpawnKind::Npc(spawn.clone())
                            }
                        };

                        let msg = FromServer::ActorSpawn(
                            Actor {
                                id: *id,
                                hp: 100,
                                spawn_index: 0,
                            },
                            kind,
                        );

                        network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
                    }
                }

                let (client, _) = network.clients.get(&from_id).unwrap().clone();

                if let Some(instance) = data.find_instance(zone_id) {
                    // Then tell any clients in the zone that we spawned
                    for (id, (handle, _)) in &mut network.clients {
                        let id = *id;

                        // don't bother telling the client who told us
                        if id == from_id {
                            continue;
                        }

                        // skip any clients not in our zone
                        if !instance.actors.contains_key(&ObjectId(handle.actor_id)) {
                            continue;
                        }

                        let msg = FromServer::ActorSpawn(
                            Actor {
                                id: ObjectId(client.actor_id),
                                hp: 0,
                                spawn_index: 0,
                            },
                            SpawnKind::Player(player_spawn.clone()),
                        );

                        if handle.send(msg).is_err() {
                            to_remove.push(id);
                        }
                    }
                }

                // add the connection's actor to the table
                {
                    let instance = data.find_instance_mut(zone_id);
                    instance.actors.insert(
                        ObjectId(client.actor_id),
                        NetworkedActor::Player(player_spawn),
                    );
                }
            }
            ToServer::ChangeZone(from_id, actor_id, zone_id) => {
                tracing::info!("Client {from_id:?} is requesting to go to {zone_id}!");

                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();

                // create a new instance if necessary
                let mut game_data = game_data.lock().unwrap();
                data.ensure_exists(zone_id, &mut game_data);

                // inform the players in this zone that this actor left
                if let Some(current_instance) = data.find_actor_instance_mut(actor_id) {
                    current_instance.actors.remove(&ObjectId(actor_id));
                    network.inform_remove_actor(current_instance, from_id, actor_id);
                }

                // then find or create a new instance with the zone id
                data.ensure_exists(zone_id, &mut game_data);
                let target_instance = data.find_instance_mut(zone_id);

                // tell the client to load into the zone
                let msg = FromServer::ChangeZone(
                    zone_id,
                    target_instance.weather_id,
                    Position::default(),
                    0.0,
                    target_instance.zone.to_lua_zone(target_instance.weather_id),
                    false,
                );
                network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
            }
            ToServer::EnterZoneJump(from_id, actor_id, exitbox_id) => {
                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();
                let mut game_data = game_data.lock().unwrap();

                // first, find the zone jump in the current zone
                let destination_zone_id;
                let destination_instance_id;
                if let Some(current_instance) = data.find_actor_instance(actor_id) {
                    let Some((_, new_exit_box)) = current_instance.zone.find_exit_box(exitbox_id)
                    else {
                        tracing::warn!("Couldn't find exit box {exitbox_id}?!");
                        break;
                    };
                    destination_zone_id = new_exit_box.territory_type;
                    destination_instance_id = new_exit_box.destination_instance_id;
                } else {
                    tracing::warn!(
                        "Actor isn't in the instance it was expected in. This is a bug!"
                    );
                    break;
                }

                do_change_zone(
                    &mut data,
                    &mut network,
                    &mut game_data,
                    destination_zone_id,
                    destination_instance_id,
                    actor_id,
                    from_id,
                );
            }
            ToServer::Warp(from_id, actor_id, warp_id) => {
                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();
                let mut game_data = game_data.lock().unwrap();

                // first, find the warp and it's destination
                let (destination_instance_id, destination_zone_id) = game_data
                    .get_warp(warp_id)
                    .expect("Failed to find the warp!");

                do_change_zone(
                    &mut data,
                    &mut network,
                    &mut game_data,
                    destination_zone_id,
                    destination_instance_id,
                    actor_id,
                    from_id,
                );
            }
            ToServer::WarpAetheryte(from_id, actor_id, aetheryte_id) => {
                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();
                let mut game_data = game_data.lock().unwrap();

                // first, find the warp and it's destination
                let (destination_instance_id, destination_zone_id) = game_data
                    .get_aetheryte(aetheryte_id)
                    .expect("Failed to find the aetheryte!");

                do_change_zone(
                    &mut data,
                    &mut network,
                    &mut game_data,
                    destination_zone_id,
                    destination_instance_id,
                    actor_id,
                    from_id,
                );
            }
            ToServer::Message(from_id, msg) => {
                let mut network = network.lock().unwrap();

                network.send_to_all(
                    Some(from_id),
                    FromServer::Message(msg),
                    DestinationNetwork::ZoneClients,
                );
            }
            ToServer::ActorMoved(
                from_id,
                actor_id,
                position,
                rotation,
                anim_type,
                anim_state,
                jump_state,
            ) => {
                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();

                if let Some(instance) = data.find_actor_instance_mut(actor_id) {
                    if let Some((_, spawn)) = instance
                        .actors
                        .iter_mut()
                        .find(|actor| *actor.0 == ObjectId(actor_id))
                    {
                        let common = match spawn {
                            NetworkedActor::Player(npc_spawn) => &mut npc_spawn.common,
                            NetworkedActor::Npc { spawn, .. } => &mut spawn.common,
                        };
                        common.pos = position;
                        common.rotation = rotation;
                    }

                    let msg = FromServer::ActorMove(
                        actor_id, position, rotation, anim_type, anim_state, jump_state,
                    );
                    network.send_to_all(Some(from_id), msg, DestinationNetwork::ZoneClients);
                }
            }
            ToServer::ClientTrigger(from_id, from_actor_id, trigger) => {
                let mut network = network.lock().unwrap();

                for (id, (handle, _)) in &mut network.clients {
                    let id = *id;

                    tracing::info!("{:#X?}", trigger);

                    // handle player-to-server actions
                    if id == from_id {
                        if let ClientTriggerCommand::TeleportQuery { aetheryte_id } =
                            &trigger.trigger
                        {
                            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                                category: ActorControlCategory::TeleportStart {
                                    insufficient_gil: 0,
                                    aetheryte_id: *aetheryte_id,
                                },
                            });

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }

                        if let ClientTriggerCommand::EventRelatedUnk { .. } = &trigger.trigger {
                            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                                category: ActorControlCategory::EventRelatedUnk1 { unk1: 1 },
                            });

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                                category: ActorControlCategory::EventRelatedUnk2 { unk1: 0 },
                            });

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }

                        if let ClientTriggerCommand::WalkInTriggerFinished { .. } = &trigger.trigger
                        {
                            // This is where we finally release the client after the walk-in trigger.
                            let msg = FromServer::Conditions(Conditions::default());

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }

                            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                                category: ActorControlCategory::WalkInTriggerRelatedUnk1 {
                                    unk1: 0,
                                },
                            });

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }

                            // Yes, this is actually sent every time the trigger event finishes...
                            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                                category: ActorControlCategory::CompanionUnlock {
                                    unk1: 0,
                                    unk2: 1,
                                },
                            });

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }

                            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                                category: ActorControlCategory::WalkInTriggerRelatedUnk2 {
                                    unk1: 0,
                                    unk2: 0,
                                    unk3: 0,
                                    unk4: 7,
                                },
                            });

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }

                        if let ClientTriggerCommand::SummonMinion { minion_id } = &trigger.trigger {
                            let msg = FromServer::ActorSummonsMinion(*minion_id);

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }

                        if let ClientTriggerCommand::DespawnMinion { .. } = &trigger.trigger {
                            let msg = FromServer::ActorDespawnsMinion();

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        continue;
                    }

                    match &trigger.trigger {
                        ClientTriggerCommand::SetTarget {
                            actor_id,
                            actor_type,
                        } => {
                            // For whatever reason these don't match what the server has to send back, so they cannot be directly reused.
                            let actor_type = match *actor_type {
                                0 => ObjectTypeKind::None,
                                1 => ObjectTypeKind::EObjOrNpc,
                                2 => ObjectTypeKind::Minion,
                                _ => {
                                    // TODO: Are there other types?
                                    tracing::warn!(
                                        "SetTarget: Unknown actor target type {}! Defaulting to None!",
                                        *actor_type
                                    );
                                    ObjectTypeKind::None
                                }
                            };

                            let msg = FromServer::ActorControlTarget(
                                from_actor_id,
                                ActorControlTarget {
                                    category: ActorControlCategory::SetTarget {
                                        target: ObjectTypeId {
                                            object_id: *actor_id,
                                            object_type: actor_type,
                                        },
                                    },
                                },
                            );

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        ClientTriggerCommand::ChangePose { unk1, pose } => {
                            let msg = FromServer::ActorControl(
                                from_actor_id,
                                ActorControl {
                                    category: ActorControlCategory::Pose {
                                        unk1: *unk1,
                                        pose: *pose,
                                    },
                                },
                            );

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        ClientTriggerCommand::ReapplyPose { unk1, pose } => {
                            let msg = FromServer::ActorControl(
                                from_actor_id,
                                ActorControl {
                                    category: ActorControlCategory::Pose {
                                        unk1: *unk1,
                                        pose: *pose,
                                    },
                                },
                            );

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        ClientTriggerCommand::Emote(emote_info) => {
                            let msg = FromServer::ActorControlTarget(
                                from_actor_id,
                                ActorControlTarget {
                                    category: ActorControlCategory::Emote(*emote_info),
                                },
                            );

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        ClientTriggerCommand::ToggleWeapon { shown, unk_flag } => {
                            let msg = FromServer::ActorControl(
                                from_actor_id,
                                ActorControl {
                                    category: ActorControlCategory::ToggleWeapon {
                                        shown: *shown,
                                        unk_flag: *unk_flag,
                                    },
                                },
                            );

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        ClientTriggerCommand::ManuallyRemoveEffect {
                            effect_id,
                            source_actor_id,
                            ..
                        } => {
                            // TODO: we need to inform the ZoneConnection as well since it keeps track of its own status effect list...

                            let msg =
                                FromServer::LoseEffect(*effect_id as u16, 0, *source_actor_id);

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        _ => tracing::warn!("Server doesn't know what to do with {:#?}", trigger),
                    }
                }
            }
            ToServer::DebugNewNpc(_from_id, from_actor_id) => {
                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();

                let actor_id = Instance::generate_actor_id();
                let spawn;
                {
                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        break;
                    };

                    let Some(actor) = instance.find_actor(ObjectId(from_actor_id)) else {
                        break;
                    };

                    let NetworkedActor::Player(player) = actor else {
                        break;
                    };

                    spawn = NpcSpawn {
                        aggression_mode: 1,
                        common: CommonSpawn {
                            hp_curr: 100,
                            hp_max: 100,
                            mp_curr: 100,
                            mp_max: 100,
                            look: CUSTOMIZE_DATA,
                            bnpc_base: 13498,
                            bnpc_name: 10261,
                            object_kind: ObjectKind::BattleNpc(BattleNpcSubKind::Enemy),
                            target_id: ObjectTypeId {
                                object_id: ObjectId(from_actor_id),
                                object_type: ObjectTypeKind::None,
                            }, // target the player
                            level: 1,
                            models: [
                                0,  // head
                                89, // body
                                89, // hands
                                89, // legs
                                89, // feet
                                0,  // ears
                                0,  // neck
                                0,  // wrists
                                0,  // left finger
                                0,  // right finger
                            ],
                            pos: player.common.pos,
                            ..Default::default()
                        },
                        ..Default::default()
                    };

                    instance.insert_npc(ObjectId(actor_id), spawn.clone());
                }

                network.send_actor(
                    Actor {
                        id: ObjectId(actor_id),
                        ..Default::default()
                    },
                    SpawnKind::Npc(spawn),
                );
            }
            ToServer::DebugNewEnemy(_from_id, from_actor_id, id) => {
                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();

                let actor_id = Instance::generate_actor_id();
                let spawn;
                {
                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        break;
                    };

                    let Some(actor) = instance.find_actor(ObjectId(from_actor_id)) else {
                        break;
                    };

                    let NetworkedActor::Player(player) = actor else {
                        break;
                    };

                    let model_chara;
                    {
                        let mut game_data = game_data.lock().unwrap();
                        model_chara = game_data.find_bnpc(id).unwrap();
                    }

                    spawn = NpcSpawn {
                        aggression_mode: 1,
                        common: CommonSpawn {
                            hp_curr: 91,
                            hp_max: 91,
                            mp_curr: 100,
                            mp_max: 100,
                            spawn_index: 0, // not needed at this level
                            bnpc_base: id,
                            bnpc_name: 405,
                            object_kind: ObjectKind::BattleNpc(BattleNpcSubKind::Enemy),
                            level: 1,
                            battalion: 4,
                            model_chara,
                            pos: player.common.pos,
                            ..Default::default()
                        },
                        ..Default::default()
                    };

                    instance.insert_npc(ObjectId(actor_id), spawn.clone());
                }

                network.send_actor(
                    Actor {
                        id: ObjectId(actor_id),
                        ..Default::default()
                    },
                    SpawnKind::Npc(spawn),
                );
            }
            ToServer::DebugSpawnClone(_from_id, from_actor_id) => {
                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();

                let actor_id = Instance::generate_actor_id();
                let spawn;
                {
                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        break;
                    };

                    let Some(actor) = instance.find_actor(ObjectId(from_actor_id)) else {
                        break;
                    };

                    let NetworkedActor::Player(player) = actor else {
                        break;
                    };

                    spawn = NpcSpawn {
                        aggression_mode: 1,
                        common: player.common.clone(),
                        ..Default::default()
                    };

                    instance.insert_npc(ObjectId(actor_id), spawn.clone());
                }

                network.send_actor(
                    Actor {
                        id: ObjectId(actor_id),
                        ..Default::default()
                    },
                    SpawnKind::Npc(spawn),
                );
            }
            ToServer::ActionRequest(from_id, _from_actor_id, request) => {
                let mut game_data = game_data.lock().unwrap();
                let cast_time = game_data.get_casttime(request.action_key).unwrap();

                let send_execution = |from_id: ClientId, network: Arc<Mutex<NetworkState>>| {
                    let mut network = network.lock().unwrap();

                    tracing::info!("Now finishing delayed cast!");

                    let msg = FromServer::ActionComplete(request);
                    network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
                };

                if cast_time == 0 {
                    // If instantaneous, send right back
                    send_execution(from_id, network.clone());
                } else {
                    // Otherwise, delay
                    // NOTE: I know this won't scale, but it's a fine hack for now

                    tracing::info!(
                        "Delaying spell cast for {} milliseconds",
                        cast_time as u64 * 100
                    );

                    // we have to shadow these variables to tell rust not to move them into the async closure
                    let network = network.clone();
                    tokio::task::spawn(async move {
                        let mut interval =
                            tokio::time::interval(Duration::from_millis(cast_time as u64 * 100));
                        interval.tick().await;
                        interval.tick().await;
                        send_execution(from_id, network);
                    });
                }
            }
            ToServer::Config(_from_id, from_actor_id, config) => {
                // update their stored state so it's correctly sent on new spawns
                {
                    let mut data = data.lock().unwrap();

                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        break;
                    };

                    let Some(actor) = instance.find_actor_mut(ObjectId(from_actor_id)) else {
                        break;
                    };

                    let NetworkedActor::Player(player) = actor else {
                        break;
                    };

                    player.common.display_flags = config.display_flag.into();
                }

                let mut network = network.lock().unwrap();
                let msg = FromServer::UpdateConfig(from_actor_id, config.clone());

                network.send_to_all(None, msg, DestinationNetwork::ZoneClients);
            }
            ToServer::Equip(_from_id, from_actor_id, main_weapon_id, sub_weapon_id, model_ids) => {
                // update their stored state so it's correctly sent on new spawns
                {
                    let mut data = data.lock().unwrap();

                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        break;
                    };

                    let Some(actor) = instance.find_actor_mut(ObjectId(from_actor_id)) else {
                        break;
                    };

                    let NetworkedActor::Player(player) = actor else {
                        break;
                    };

                    player.common.main_weapon_model = main_weapon_id;
                    player.common.sec_weapon_model = sub_weapon_id;
                    player.common.models = model_ids;
                }

                // Inform all clients about their new equipped model ids
                let msg =
                    FromServer::ActorEquip(from_actor_id, main_weapon_id, sub_weapon_id, model_ids);

                let mut network = network.lock().unwrap();
                network.send_to_all(None, msg, DestinationNetwork::ZoneClients);
            }
            ToServer::BeginReplay(from_id, path) => {
                let mut entries = std::fs::read_dir(path)
                    .unwrap()
                    .map(|res| res.map(|e| e.path()))
                    .collect::<Result<Vec<_>, std::io::Error>>()
                    .unwrap();

                entries.sort_by(|a, b| {
                    let a_seq = a
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .split_once('-')
                        .unwrap()
                        .0
                        .parse::<i32>()
                        .unwrap();
                    let b_seq = b
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .split_once('-')
                        .unwrap()
                        .0
                        .parse::<i32>()
                        .unwrap();

                    a_seq.cmp(&b_seq)
                });

                let send_execution = |from_id: ClientId,
                                      network: Arc<Mutex<NetworkState>>,
                                      entry: &PathBuf| {
                    let mut network = network.lock().unwrap();

                    // only care about Ipc packets
                    let filename = entry.file_name().unwrap().to_str().unwrap();
                    if !filename.contains("Ipc") {
                        return;
                    }

                    // only care about packets from the server
                    if !filename.contains("(to client)") {
                        return;
                    }

                    let path = entry.to_str().unwrap();

                    tracing::info!("- Replaying {path}");

                    let source_actor_bytes =
                        std::fs::read(format!("{path}/source_actor.bin")).unwrap();
                    let target_actor_bytes =
                        std::fs::read(format!("{path}/target_actor.bin")).unwrap();
                    let source_actor =
                        u32::from_le_bytes(source_actor_bytes[0..4].try_into().unwrap());
                    let target_actor =
                        u32::from_le_bytes(target_actor_bytes[0..4].try_into().unwrap());

                    let ipc_header_bytes = std::fs::read(format!("{path}/ipc_header.bin")).unwrap();
                    let opcode = u16::from_le_bytes(ipc_header_bytes[2..4].try_into().unwrap());

                    let mut ipc_data = std::fs::read(format!("{path}/data.bin")).unwrap();
                    let ipc_len = ipc_data.len() as u32 + 32;
                    let mut cursor = Cursor::new(&mut ipc_data);
                    if let Ok(parsed) = ServerZoneIpcSegment::read_le_args(&mut cursor, (&ipc_len,))
                        && let ServerZoneIpcData::InitZone(mut init_zone) = parsed.data
                    {
                        tracing::info!("- Fixing up InitZone");

                        // stop it from trying to initialize obsfucation
                        init_zone.obsfucation_mode = 0;
                        init_zone.seed1 = 0;
                        init_zone.seed2 = 0;
                        init_zone.seed3 = 0;

                        let mut cursor = Cursor::new(Vec::new());
                        init_zone.write_le(&mut cursor).unwrap();
                        ipc_data = cursor.into_inner().to_vec();
                    }

                    let msg = FromServer::ReplayPacket(PacketSegment {
                        source_actor,
                        target_actor,
                        segment_type: SegmentType::Ipc,
                        data: SegmentData::Ipc(ServerZoneIpcSegment {
                            header: ServerIpcSegmentHeader::from_opcode(
                                ServerZoneIpcType::Unknown(opcode),
                            ),
                            data: ServerZoneIpcData::Unknown { unk: ipc_data },
                        }),
                    });
                    network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
                };

                let network = network.clone();
                tokio::task::spawn(async move {
                    for entry in &entries {
                        let mut interval = tokio::time::interval(Duration::from_millis(100));
                        interval.tick().await;
                        interval.tick().await;
                        send_execution(from_id, network.clone(), entry);
                    }
                });
            }
            ToServer::GainEffect(
                from_id,
                _from_actor_id,
                effect_id,
                effect_duration,
                effect_param,
                effect_source_actor_id,
            ) => {
                let send_lost_effect =
                    |from_id: ClientId,
                     network: Arc<Mutex<NetworkState>>,
                     effect_id: u16,
                     effect_param: u16,
                     effect_source_actor_id: ObjectId| {
                        let mut network = network.lock().unwrap();

                        tracing::info!("Now losing effect {}!", effect_id);

                        let msg =
                            FromServer::LoseEffect(effect_id, effect_param, effect_source_actor_id);
                        network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
                    };

                // Eventually tell the player they lost this effect
                // NOTE: I know this won't scale, but it's a fine hack for now

                tracing::info!("Effect {effect_id} lasts for {effect_duration} seconds");

                // we have to shadow these variables to tell rust not to move them into the async closure
                let network = network.clone();
                tokio::task::spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_millis(
                        (effect_duration * 1000.0) as u64,
                    ));
                    interval.tick().await;
                    interval.tick().await;
                    send_lost_effect(
                        from_id,
                        network,
                        effect_id,
                        effect_param,
                        effect_source_actor_id,
                    );
                });
            }
            ToServer::ZoneIn(from_id, from_actor_id, is_teleport) => {
                // Inform all clients to play the zone in animation
                let mut network = network.lock().unwrap();
                for (id, (handle, _)) in &mut network.clients {
                    let id = *id;

                    let category = ActorControlCategory::ZoneIn {
                        warp_finish_anim: 1,
                        raise_anim: 0,
                        unk1: if is_teleport { 110 } else { 0 },
                    };

                    if id == from_id {
                        let msg = FromServer::ActorControlSelf(ActorControlSelf { category });

                        if handle.send(msg).is_err() {
                            to_remove.push(id);
                        }
                    } else {
                        let msg =
                            FromServer::ActorControl(from_actor_id, ActorControl { category });

                        if handle.send(msg).is_err() {
                            to_remove.push(id);
                        }
                    }
                }
            }
            ToServer::Disconnected(from_id, from_actor_id) => {
                let mut network = network.lock().unwrap();
                network.to_remove.push(from_id);

                // Tell our sibling chat connection that it's time to go too.
                network.send_to_by_actor_id(
                    from_actor_id,
                    FromServer::ChatDisconnected(),
                    DestinationNetwork::ChatClients,
                );
            }
            ToServer::ActorSummonsMinion(from_id, from_actor_id, minion_id) => {
                let mut network = network.lock().unwrap();
                let mut data = data.lock().unwrap();

                set_player_minion(
                    &mut data,
                    &mut network,
                    &mut to_remove,
                    minion_id,
                    from_id,
                    from_actor_id,
                );
            }
            ToServer::ActorDespawnsMinion(from_id, from_actor_id) => {
                let mut network = network.lock().unwrap();
                let mut data = data.lock().unwrap();

                set_player_minion(
                    &mut data,
                    &mut network,
                    &mut to_remove,
                    0,
                    from_id,
                    from_actor_id,
                );
            }
            ToServer::MoveToPopRange(from_id, from_actor_id, id) => {
                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();

                tracing::info!("finding {id}");

                if let Some(instance) = data.find_actor_instance_mut(from_actor_id) {
                    if let Some(pop_range) = instance.zone.find_pop_range(id) {
                        let trans = pop_range.0.transform.translation;

                        let msg = FromServer::NewPosition(Position {
                            x: trans[0],
                            y: trans[1],
                            z: trans[2],
                        });

                        // send new position to the client
                        network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
                    } else {
                        tracing::warn!("Failed to find pop range for {id}!");
                    }
                }
            }
            ToServer::TellMessageSent(from_id, from_actor_id, message_info) => {
                // TODO: Maybe this can be simplified with fewer loops?

                let mut network = network.lock().unwrap();
                let data = data.lock().unwrap();

                // First pull up some info about the sender, as tell packets require it
                let Some(sender_instance) = data.find_actor_instance(from_actor_id) else {
                    panic!("ToServer::TellMessageSent: Unable to find the sender! What happened?");
                };

                let mut sender_name = "".to_string();
                let mut sender_world_id = 0;
                let mut sender_account_id = 0;

                for (id, actor) in &sender_instance.actors {
                    if id.0 == from_actor_id {
                        let Some(spawn) = actor.get_player_spawn() else {
                            panic!("Why are we trying to get the PlayerSpawn of an NPC?");
                        };

                        sender_name = spawn.common.name.clone();
                        sender_world_id = spawn.home_world_id;
                        sender_account_id = spawn.account_id;
                        break;
                    }
                }

                // If the sender wasn't found in the instance we already found them to be in, reality has apparently broken
                assert!(sender_world_id != 0);

                let mut recipient_actor_id = INVALID_OBJECT_ID;

                // Second, look up the recipient by name, since that and their world id are all we're given by the sending client.
                // Since we don't implement multiple worlds, the world id isn't useful for anything here.
                'outer: for instance in data.instances.values() {
                    for (id, actor) in &instance.actors {
                        if actor.get_common_spawn().name == message_info.recipient_name {
                            recipient_actor_id = *id;
                            break 'outer;
                        }
                    }
                }

                // Next, if the recipient is online, fetch their handle from the network and send them the message!
                if recipient_actor_id != INVALID_OBJECT_ID {
                    let message_info = MessageInfo {
                        sender_actor_id: from_actor_id,
                        sender_account_id,
                        sender_name: sender_name.clone(),
                        sender_world_id,
                        message: message_info.message.clone(),
                        ..Default::default()
                    };

                    network.send_to_by_actor_id(
                        recipient_actor_id.0,
                        FromServer::TellMessageSent(message_info),
                        DestinationNetwork::ChatClients,
                    );
                } else {
                    // Else, if the recipient is offline, inform the sender.
                    let response = TellNotFoundError {
                        sender_account_id,
                        recipient_world_id: sender_world_id, // It doesn't matter if it's the sender's, since we don't implement multiple worlds.
                        recipient_name: message_info.recipient_name.clone(),
                        ..Default::default()
                    };

                    network.send_to(
                        from_id,
                        FromServer::TellRecipientNotFound(response),
                        DestinationNetwork::ChatClients,
                    );
                }
            }

            ToServer::InvitePlayerToParty(from_actor_id, content_id, character_name) => {
                // TODO: Return an error when the target player's already in a party or offline somehow
                let mut network = network.lock().unwrap();
                let data = data.lock().unwrap();

                // First pull up some info about the sender, as tell packets require it
                let Some(sender_instance) = data.find_actor_instance(from_actor_id.0) else {
                    tracing::error!(
                        "ToServer::InvitePlayerToParty: Unable to find the sender! What happened?"
                    );
                    continue;
                };

                let mut sender_name = "".to_string();
                let mut sender_account_id = 0;
                let mut sender_content_id = 0;

                for (id, actor) in &sender_instance.actors {
                    if id.0 == from_actor_id.0 {
                        let Some(spawn) = actor.get_player_spawn() else {
                            panic!("Why are we trying to get the PlayerSpawn of an NPC?");
                        };

                        sender_name = spawn.common.name.clone();
                        sender_account_id = spawn.account_id;
                        sender_content_id = spawn.content_id;
                        break;
                    }
                }

                // If the sender wasn't found in the instance we already found them to be in, reality has apparently broken
                assert!(sender_content_id != 0);

                let mut recipient_actor_id = INVALID_OBJECT_ID;

                // Second, look up the recipient by name, since that and their content id are all we're given by the sending client.
                // Since we don't implement multiple worlds, the world id isn't useful for anything here.
                'outer: for instance in data.instances.values() {
                    for (id, actor) in &instance.actors {
                        if let NetworkedActor::Player(spawn) = actor {
                            if spawn.content_id == content_id || spawn.common.name == character_name
                            {
                                recipient_actor_id = *id;
                                break 'outer;
                            }
                        }
                    }
                }

                let mut already_in_party = false;

                // Next, see if the recipient is already in a party, and let the sender know if they are...
                'outer: for party in network.parties.values() {
                    for member in &party.members {
                        if member.actor_id == recipient_actor_id {
                            already_in_party = true;
                            break 'outer;
                        }
                    }
                }

                if !already_in_party {
                    // Finally, if the recipient is online, fetch their handle from the network and send them the message!
                    if recipient_actor_id != INVALID_OBJECT_ID {
                        for (id, (handle, _)) in &mut network.clients {
                            if handle.actor_id == recipient_actor_id.0 {
                                let msg = FromServer::PartyInvite(
                                    sender_account_id,
                                    sender_content_id,
                                    sender_name,
                                );
                                if handle.send(msg.clone()).is_err() {
                                    to_remove.push(*id);
                                }
                                break;
                            }
                        }
                    } else {
                        // TODO: Else, if the recipient is offline, inform the sender.
                        tracing::error!(
                            "InvitePlayerToParty: The recipient is offline! What happened?"
                        );
                    }
                } else {
                    let msg = FromServer::CharacterAlreadyInParty();
                    network.send_to_by_actor_id(
                        from_actor_id.0,
                        msg,
                        DestinationNetwork::ZoneClients,
                    );
                }
            }
            ToServer::InvitationResponse(
                from_id,
                from_account_id,
                from_content_id,
                from_name,
                sender_content_id,
                invite_type,
                response,
            ) => {
                let mut network = network.lock().unwrap();
                let data = data.lock().unwrap();

                // Look up the invite sender and tell them the response.
                let mut recipient_actor_id = INVALID_OBJECT_ID;

                // Second, look up the recipient (the original invite sender) by content id, since that is all we're given by the sending client.
                'outer: for instance in data.instances.values() {
                    for (id, actor) in &instance.actors {
                        let Some(spawn) = actor.get_player_spawn() else {
                            panic!("Why are we looking up the PlayerSpawn of an NPC?");
                        };
                        if spawn.content_id == sender_content_id {
                            recipient_actor_id = *id;
                            break 'outer;
                        }
                    }
                }

                if recipient_actor_id != INVALID_OBJECT_ID {
                    for (id, (handle, _)) in &mut network.clients {
                        // Tell the invite sender about the invite result
                        if handle.actor_id == recipient_actor_id.0
                            && recipient_actor_id != INVALID_OBJECT_ID
                        {
                            let msg = FromServer::InvitationResult(
                                from_account_id,
                                from_content_id,
                                from_name.clone(),
                                invite_type,
                                response,
                            );
                            if handle.send(msg.clone()).is_err() {
                                to_remove.push(*id);
                            }
                        }
                        // Tell the client who just responded to the sender's invite to wait for further instructions
                        if *id == from_id {
                            let msg = FromServer::InvitationReplyResult(
                                from_content_id,
                                from_name.clone(),
                                invite_type,
                                response,
                            );
                            if handle.send(msg.clone()).is_err() {
                                to_remove.push(*id);
                            }
                        }
                    }
                }
            }
            ToServer::RequestSocialList(from_id, from_actor_id, from_party_id, request) => {
                let mut network = network.lock().unwrap();
                let data = data.lock().unwrap();
                let mut entries = vec![PlayerEntry::default(); 10];

                match &request.request_type {
                    SocialListRequestType::Party => {
                        if from_party_id != 0 {
                            let leader_actor_id = network.parties[&from_party_id].leader_id;
                            let mut index: usize = 0;
                            for member in &network.parties[&from_party_id].members {
                                // The internal party list can and will contain invalid entries representing empty slots, so skip them.
                                if !member.is_valid() {
                                    continue;
                                }

                                if !member.is_online() {
                                    entries[index].content_id = member.content_id;
                                    entries[index].name = member.name.clone();
                                    entries[index].current_world_id = 65535; // This doesn't seem to matter, but retail does it.
                                    entries[index].ui_flags =
                                        SocialListUIFlags::ENABLE_CONTEXT_MENU;
                                    entries[index].home_world_id = member.world_id;
                                    index += 1;
                                    continue;
                                }

                                let Some(instance) = data.find_actor_instance(member.actor_id.0)
                                else {
                                    // TOOD: This situation might be panic-worthy? Reality should have broken, or an invalid party member slipped past the earlier check if this trips.
                                    tracing::error!(
                                        "Unable to find this actor in any instance, what happened? {} {}",
                                        member.actor_id.0,
                                        member.name.clone()
                                    );
                                    continue;
                                };

                                for (id, actor) in &instance.actors {
                                    if *id == member.actor_id {
                                        let Some(spawn) = actor.get_player_spawn() else {
                                            panic!(
                                                "Why are we trying to get the PlayerSpawn of an NPC?"
                                            );
                                        };
                                        let mut online_status_mask = OnlineStatusMask::default();
                                        online_status_mask.set_status(OnlineStatus::Online);
                                        online_status_mask.set_status(OnlineStatus::PartyMember);
                                        if member.actor_id.0 == leader_actor_id {
                                            online_status_mask
                                                .set_status(OnlineStatus::PartyLeader);
                                        }
                                        entries[index].online_status_mask = online_status_mask;
                                        entries[index].classjob_id = spawn.common.class_job;
                                        entries[index].classjob_level = spawn.common.level;
                                        entries[index].zone_id = instance.zone.id;

                                        entries[index].content_id = spawn.content_id;
                                        entries[index].home_world_id = member.world_id;
                                        entries[index].current_world_id = spawn.current_world_id;
                                        entries[index].name = spawn.common.name.clone();
                                        entries[index].ui_flags =
                                            SocialListUIFlags::ENABLE_CONTEXT_MENU;

                                        index += 1;
                                        break;
                                    }
                                }
                            }
                        } else {
                            let Some(instance) = data.find_actor_instance(from_actor_id) else {
                                continue;
                            };

                            for (id, actor) in &instance.actors {
                                if id.0 == from_actor_id {
                                    let Some(spawn) = actor.get_player_spawn() else {
                                        panic!(
                                            "Why are we trying to get the PlayerSpawn of an NPC?"
                                        );
                                    };

                                    // TODO: Probably start with a cached status from elsewhere?
                                    let mut online_status_mask = OnlineStatusMask::default();
                                    online_status_mask.set_status(OnlineStatus::Online);

                                    entries[0].content_id = spawn.content_id;
                                    entries[0].current_world_id = spawn.home_world_id;
                                    entries[0].home_world_id = spawn.home_world_id;
                                    entries[0].name = spawn.common.name.clone();
                                    entries[0].ui_flags = SocialListUIFlags::ENABLE_CONTEXT_MENU;
                                    entries[0].online_status_mask = online_status_mask;
                                    entries[0].classjob_id = spawn.common.class_job;
                                    entries[0].classjob_level = spawn.common.level;
                                    entries[0].zone_id = instance.zone.id;
                                    break;
                                }
                            }
                        }
                    }
                    SocialListRequestType::Friends => {
                        tracing::warn!(
                            "SocialListRequestType was Friends! This is not yet implemented!"
                        );
                    }
                }

                let msg =
                    FromServer::SocialListResponse(request.request_type, request.count, entries);
                network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
            }
            ToServer::AddPartyMember(party_id, leader_actor_id, new_member_content_id) => {
                let mut network = network.lock().unwrap();
                let data = data.lock().unwrap();
                let mut party_id = party_id;

                // This client is creating a party.
                if party_id == 0 {
                    // TODO: We should probably generate these differently so there are no potential collisions.
                    party_id = fastrand::u64(..);
                    let chatchannel_id = fastrand::u32(..);
                    let party = network.parties.entry(party_id).or_default();
                    party.chatchannel_id = chatchannel_id;
                    party.leader_id = leader_actor_id;
                    party.members[0].actor_id = ObjectId(leader_actor_id);
                }

                if let Some(party) = network.parties.get(&party_id) {
                    let chatchannel_id = network.parties[&party_id].chatchannel_id;
                    let mut party = party.members.clone();

                    let mut party_list = Vec::<PartyMemberEntry>::new();

                    let mut execute_account_id = 0;
                    let mut execute_content_id = 0;
                    let mut execute_name = String::default();
                    let mut target_account_id = 0;
                    let mut target_content_id = 0;
                    let mut target_name = String::default();

                    // TODO: This can probably be simplified/the logic can probably be adjusted, need to think more on this
                    for instance in data.instances.values() {
                        for (id, actor) in &instance.actors {
                            let Some(spawn) = actor.get_player_spawn() else {
                                continue;
                            };

                            if spawn.content_id == new_member_content_id {
                                // Find the first open member slot.
                                let Some(free_index) =
                                    party.iter().position(|x| x.actor_id == INVALID_OBJECT_ID)
                                else {
                                    // TODO: See if we can gracefully exit from here without a panic
                                    panic!(
                                        "Tried to add a party member to a full party! What happened? {party:#?}"
                                    );
                                };
                                party[free_index].actor_id = *id;
                                target_account_id = spawn.account_id;
                                target_content_id = spawn.content_id;
                                target_name = spawn.common.name.clone();
                            }

                            if id.0 == leader_actor_id {
                                execute_account_id = spawn.account_id;
                                execute_content_id = spawn.content_id;
                                execute_name = spawn.common.name.clone();
                            }

                            for member in &mut party {
                                if member.actor_id == *id {
                                    member.account_id = spawn.account_id;
                                    member.content_id = spawn.content_id;
                                    member.name = spawn.common.name.clone();

                                    party_list.push(PartyMemberEntry {
                                        account_id: spawn.account_id,
                                        content_id: spawn.content_id,
                                        name: spawn.common.name.clone(),
                                        actor_id: *id,
                                        classjob_id: spawn.common.class_job,
                                        classjob_level: spawn.common.level,
                                        current_hp: spawn.common.hp_curr,
                                        max_hp: spawn.common.hp_max,
                                        current_mp: spawn.common.mp_curr,
                                        max_mp: spawn.common.mp_max,
                                        current_zone_id: instance.zone.id,
                                        home_world_id: spawn.home_world_id,
                                        ..Default::default()
                                    });
                                    break;
                                }
                            }
                        }
                    }

                    assert!(
                        !party_list.is_empty() && party_list.len() <= PartyMemberEntry::NUM_ENTRIES
                    );

                    let update_status = PartyUpdateStatus::JoinParty;

                    let msg = FromServer::PartyUpdate(
                        PartyUpdateTargets {
                            execute_account_id,
                            execute_content_id,
                            execute_name: execute_name.clone(),
                            target_account_id,
                            target_content_id,
                            target_name: target_name.clone(),
                        },
                        update_status,
                        Some((
                            party_id,
                            chatchannel_id,
                            ObjectId(leader_actor_id),
                            party_list.clone(),
                        )),
                    );

                    // Next, tell everyone in the party someone joined (including the joining player themself).
                    // Also cache their client ids to speed up sending future replies.
                    for (id, (handle, _)) in &mut network.clients {
                        for member in &mut party {
                            if member.actor_id.0 == handle.actor_id {
                                member.zone_client_id = *id;
                                if handle.send(msg.clone()).is_err() {
                                    to_remove.push(*id);
                                }
                            }
                        }
                    }

                    let msg = FromServer::SetPartyChatChannel(chatchannel_id);

                    // Finally, tell their chat connections they're now in a party.
                    // Also cache their client ids to speed up sending future replies.
                    for (id, (handle, _)) in &mut network.chat_clients {
                        for member in &mut party {
                            if member.actor_id.0 == handle.actor_id {
                                member.chat_client_id = *id;
                                if handle.send(msg.clone()).is_err() {
                                    to_remove.push(*id);
                                }
                            }
                        }
                    }

                    network.parties.get_mut(&party_id).unwrap().members = party; // Now we can give the clone back after all that nonsense
                } else {
                    tracing::error!(
                        "AddPartyMember: Party id wasn't in the hashmap! What happened?"
                    );
                }
            }
            ToServer::PartyMessageSent(from_actor_id, message_info) => {
                let mut network = network.lock().unwrap();

                let mut sender = PartyMember::default();
                let mut party_id = 0;

                // We need some info about the sender since our chat connection doesn't provide it.
                for (id, party) in &network.parties {
                    if party.chatchannel_id == message_info.chatchannel.channel_number {
                        party_id = *id;
                        for member in &party.members {
                            if member.actor_id.0 == from_actor_id {
                                sender = member.clone();
                            }
                        }
                    }
                }

                assert!(party_id != 0 && sender.actor_id != INVALID_OBJECT_ID);

                let party_message = PartyMessage {
                    party_chatchannel: message_info.chatchannel,
                    sender_account_id: sender.account_id,
                    sender_content_id: sender.content_id,
                    sender_world_id: sender.world_id,
                    sender_actor_id: sender.actor_id.0,
                    sender_name: sender.name.clone(),
                    message: message_info.message,
                };
                let msg = FromServer::PartyMessageSent(party_message);

                // Skip the original sender to avoid echoing messages
                network.send_to_party(
                    party_id,
                    Some(from_actor_id),
                    msg,
                    DestinationNetwork::ChatClients,
                );
            }
            ToServer::PartyMemberChangedAreas(
                party_id,
                execute_account_id,
                execute_content_id,
                execute_name,
            ) => {
                let mut network = network.lock().unwrap();
                let data = data.lock().unwrap();
                let party = network.parties.get_mut(&party_id).unwrap();

                let party_list = build_party_list(party, &data);

                let msg = FromServer::PartyUpdate(
                    PartyUpdateTargets {
                        execute_account_id,
                        execute_content_id,
                        execute_name: execute_name.clone(),
                        ..Default::default()
                    },
                    PartyUpdateStatus::MemberChangedZones,
                    Some((
                        party_id,
                        party.chatchannel_id,
                        ObjectId(party.leader_id),
                        party_list,
                    )),
                );

                // Finally, tell everyone in the party about the update.
                network.send_to_party(party_id, None, msg, DestinationNetwork::ZoneClients);
            }
            ToServer::PartyChangeLeader(
                party_id,
                execute_account_id,
                execute_content_id,
                execute_name,
                target_content_id,
                target_name,
            ) => {
                let mut network = network.lock().unwrap();

                if !network.parties.contains_key(&party_id) {
                    panic!("Why are we trying to do party operations on an invalid party?");
                }

                let data = data.lock().unwrap();
                let target_account_id;
                {
                    let party = &mut network.parties.get_mut(&party_id).unwrap();
                    let Some(member) = party.get_member_by_content_id(target_content_id) else {
                        continue;
                    };
                    party.leader_id = member.actor_id.0;
                    target_account_id = member.account_id;
                }

                let party = &network.parties.get(&party_id).unwrap();

                let party_list = build_party_list(party, &data);

                let msg = FromServer::PartyUpdate(
                    PartyUpdateTargets {
                        execute_account_id,
                        execute_content_id,
                        execute_name: execute_name.clone(),
                        target_account_id,
                        target_content_id,
                        target_name: target_name.clone(),
                    },
                    PartyUpdateStatus::PromoteLeader,
                    Some((
                        party_id,
                        party.chatchannel_id,
                        ObjectId(party.leader_id),
                        party_list,
                    )),
                );

                // Finally, tell everyone in the party about the update.
                network.send_to_party(party_id, None, msg, DestinationNetwork::ZoneClients);
            }
            ToServer::PartyMemberLeft(
                party_id,
                execute_account_id,
                execute_content_id,
                execute_actor_id,
                execute_name,
            ) => {
                let mut network = network.lock().unwrap();
                let data = data.lock().unwrap();
                let party_list;
                let leaving_zone_client_id;
                let leaving_chat_client_id;
                let chatchannel_id;
                let mut leader_id;
                let member_count;
                {
                    let Some(party) = network.parties.get_mut(&party_id) else {
                        continue;
                    };
                    chatchannel_id = party.chatchannel_id;

                    // Construct the party list we're sending back to the clients in this party.
                    leaving_zone_client_id = party
                        .get_member_by_actor_id(execute_actor_id)
                        .unwrap()
                        .zone_client_id;
                    leaving_chat_client_id = party
                        .get_member_by_actor_id(execute_actor_id)
                        .unwrap()
                        .chat_client_id;

                    party.remove_member(execute_actor_id);
                    member_count = party.get_member_count();
                    leader_id = party.leader_id;

                    // If the leader left the party, and there are still enough members, auto-promote the next available player
                    if execute_actor_id == party.leader_id && member_count >= 2 {
                        leader_id = party.auto_promote_member();
                    }

                    party_list = build_party_list(party, &data);
                }

                let update_status;
                let party_info;

                if member_count < 2 {
                    update_status = PartyUpdateStatus::DisbandingParty;
                    party_info = None;
                } else {
                    update_status = PartyUpdateStatus::MemberLeftParty;
                    party_info = Some((party_id, chatchannel_id, ObjectId(leader_id), party_list));
                }

                let msg = FromServer::PartyUpdate(
                    PartyUpdateTargets {
                        execute_account_id,
                        execute_content_id,
                        execute_name: execute_name.clone(),
                        ..Default::default()
                    },
                    update_status,
                    party_info,
                );

                let leaver_msg = FromServer::PartyUpdate(
                    PartyUpdateTargets {
                        execute_account_id,
                        execute_content_id,
                        execute_name: execute_name.clone(),
                        ..Default::default()
                    },
                    update_status,
                    None,
                );

                // Tell everyone in the party about the update.
                network.send_to_party(party_id, None, msg, DestinationNetwork::ZoneClients);

                // Tell the leaver that they're not in the party anymore.
                network.send_to(
                    leaving_zone_client_id,
                    leaver_msg,
                    DestinationNetwork::ZoneClients,
                );
                network.send_to(
                    leaving_chat_client_id,
                    FromServer::SetPartyChatChannel(0),
                    DestinationNetwork::ChatClients,
                );

                // Clean up the party on our side, if necessary.
                if member_count < 2 {
                    // Tell their chat connections they're no longer in a party.
                    network.send_to_party(
                        party_id,
                        None,
                        FromServer::SetPartyChatChannel(0),
                        DestinationNetwork::ChatClients,
                    );
                    network.parties.remove(&party_id);
                }
            }
            ToServer::PartyDisband(
                party_id,
                execute_account_id,
                execute_content_id,
                execute_name,
            ) => {
                let mut network = network.lock().unwrap();

                let msg = FromServer::PartyUpdate(
                    PartyUpdateTargets {
                        execute_account_id,
                        execute_content_id,
                        execute_name: execute_name.clone(),
                        ..Default::default()
                    },
                    PartyUpdateStatus::DisbandingParty,
                    None,
                );

                // Finally, tell everyone in the party about the update.
                network.send_to_party(party_id, None, msg, DestinationNetwork::ZoneClients);

                // Tell their chat connections they're no longer in a party.
                network.send_to_party(
                    party_id,
                    None,
                    FromServer::SetPartyChatChannel(0),
                    DestinationNetwork::ChatClients,
                );

                // We don't need to keep track of this party anymore.
                network.parties.remove(&party_id);
            }
            ToServer::PartyMemberKick(
                party_id,
                execute_account_id,
                execute_content_id,
                execute_name,
                target_content_id,
                target_name,
            ) => {
                let mut network = network.lock().unwrap();
                let data = data.lock().unwrap();
                let party = network.parties.get_mut(&party_id).unwrap();

                let Some(member) = party.get_member_by_content_id(target_content_id) else {
                    continue;
                };
                party.remove_member(member.actor_id.0);

                // Construct the party list we're sending back to the clients in this party.
                let party_list = build_party_list(party, &data);

                let update_status;
                let party_info;
                let member_count = party.get_member_count();
                if member_count < 2 {
                    update_status = PartyUpdateStatus::DisbandingParty;
                    party_info = None;
                } else {
                    update_status = PartyUpdateStatus::MemberKicked;
                    party_info = Some((
                        party_id,
                        party.chatchannel_id,
                        ObjectId(party.leader_id),
                        party_list,
                    ));
                }

                let msg = FromServer::PartyUpdate(
                    PartyUpdateTargets {
                        execute_account_id,
                        execute_content_id,
                        execute_name: execute_name.clone(),
                        target_account_id: member.account_id,
                        target_content_id,
                        target_name: target_name.clone(),
                    },
                    update_status,
                    party_info,
                );

                let leaver_msg = FromServer::PartyUpdate(
                    PartyUpdateTargets {
                        execute_account_id,
                        execute_content_id,
                        execute_name: execute_name.clone(),
                        ..Default::default()
                    },
                    update_status,
                    None,
                );

                // Tell everyone in the party about the update.
                network.send_to_party(party_id, None, msg, DestinationNetwork::ZoneClients);

                // Tell the leaver that they're not in the party anymore, including their chat connection.
                network.send_to(
                    member.zone_client_id,
                    leaver_msg,
                    DestinationNetwork::ZoneClients,
                );
                network.send_to(
                    member.chat_client_id,
                    FromServer::SetPartyChatChannel(0),
                    DestinationNetwork::ChatClients,
                );

                // Clean up the party on our side, if necessary.
                if member_count < 2 {
                    // Tell their chat connections they're no longer in a party.
                    network.send_to_party(
                        party_id,
                        None,
                        FromServer::SetPartyChatChannel(0),
                        DestinationNetwork::ChatClients,
                    );
                    network.parties.remove(&party_id);
                }
            }
            ToServer::PartyMemberOffline(
                party_id,
                execute_account_id,
                execute_content_id,
                from_actor_id,
                execute_name,
            ) => {
                let mut network = network.lock().unwrap();
                let data = data.lock().unwrap();

                if !network.parties.contains_key(&party_id) {
                    tracing::error!(
                        "PartyMemberOffline: We were given an invalid party id {}. What happened?",
                        party_id
                    );
                    continue;
                }

                let party = &mut network.parties.get_mut(&party_id).unwrap();
                party.set_member_offline(from_actor_id);

                if party.get_online_member_count() > 0 {
                    let party_list = build_party_list(party, &data);

                    // Auto-promote the first available player to leader if the previous leader went offline.
                    // In this situation: retail uses PartyLeaderWentOffline as the update status, followed by sending another full MemberWentOffline update,
                    // but this is very inefficient and wasteful, so we will not do that (unless we have good reason to).
                    // The client still accepts a leader change during MemberWentOffline.
                    if party.leader_id == from_actor_id {
                        party.leader_id = party.auto_promote_member();
                    }

                    let msg = FromServer::PartyUpdate(
                        PartyUpdateTargets {
                            execute_account_id,
                            execute_content_id,
                            execute_name: execute_name.clone(),
                            ..Default::default()
                        },
                        PartyUpdateStatus::MemberWentOffline,
                        Some((
                            party_id,
                            party.chatchannel_id,
                            ObjectId(party.leader_id),
                            party_list,
                        )),
                    );

                    network.send_to_party(party_id, None, msg, DestinationNetwork::ZoneClients);
                } else {
                    // If nobody in the party is online, disband it.
                    // Retail keeps it around for ~2 hours or so if everyone is offline, but there's no point doing that.
                    network.parties.remove(&party_id);
                }
            }
            ToServer::PartyMemberReturned(execute_actor_id) => {
                let mut network = network.lock().unwrap();
                let data = data.lock().unwrap();

                let mut member = PartyMember::default();
                let mut party_id = 0;
                let mut party = Party::default();

                'outer: for (id, my_party) in &mut network.parties.iter() {
                    for my_member in &my_party.members {
                        if my_member.actor_id.0 == execute_actor_id {
                            member = my_member.clone();
                            party_id = *id;
                            party = my_party.clone();
                            break 'outer;
                        }
                    }
                }

                let party_list = build_party_list(&party, &data);
                let msg = FromServer::PartyUpdate(
                    PartyUpdateTargets {
                        execute_account_id: member.account_id,
                        execute_content_id: member.content_id,
                        execute_name: member.name.clone(),
                        ..Default::default()
                    },
                    PartyUpdateStatus::MemberReturned,
                    Some((
                        party_id,
                        party.chatchannel_id,
                        ObjectId(party.leader_id),
                        party_list,
                    )),
                );

                network.send_to_party(party_id, None, msg, DestinationNetwork::ZoneClients);
            }
            ToServer::ChatDisconnected(from_id) => {
                let mut network = network.lock().unwrap();
                network.to_remove_chat.push(from_id);
            }
            ToServer::FatalError(err) => return Err(err),
        }

        // Remove any clients that errored out
        {
            let mut network = network.lock().unwrap();
            let mut data = data.lock().unwrap();

            network.to_remove.append(&mut to_remove);

            for remove_id in network.to_remove.clone() {
                // remove any actors they had
                let mut actor_id = None;
                for (id, (handle, _)) in &mut network.clients {
                    if *id == remove_id {
                        actor_id = Some(handle.actor_id);
                    }
                }

                if let Some(actor_id) = actor_id {
                    // remove them from the instance
                    if let Some(current_instance) = data.find_actor_instance_mut(actor_id) {
                        current_instance.actors.remove(&ObjectId(actor_id));
                        network.inform_remove_actor(current_instance, remove_id, actor_id);
                    }
                }

                network.clients.remove(&remove_id);
            }

            for remove_id in network.to_remove_chat.clone() {
                network.chat_clients.remove(&remove_id);
            }
        }
    }
    Ok(())
}
