//! Managing statistics, including your classjob and other related information.

use crate::{
    GameData, ToServer, ZoneConnection,
    gamedata::{BaseParam, ParamGrow},
    inventory::Storage,
};
use kawari::{
    common::{Attributes, BASE_STAT, MAXIMUM_RESTED_EXP, ObjectId},
    ipc::zone::{
        ActorControlCategory, ActorControlSelf, PlayerStats, ServerZoneIpcData,
        ServerZoneIpcSegment, UpdateClassInfo,
    },
    packet::{PacketSegment, SegmentData, SegmentType},
};

/// Every BaseParam row, some of them may be useless.
#[derive(Default, Debug)]
pub struct BaseParameters {
    pub strength: u32,
    pub dexterity: u32,
    pub vitality: u32,
    pub intelligence: u32,
    pub mind: u32,
    pub piety: u32,
    pub hp: u32,
    pub mp: u32,
    pub tp: u32,
    pub gp: u32,
    pub cp: u32,
    pub physical_damage: u32,
    pub magic_damage: u32,
    pub delay: u32,
    pub additional_effect: u32,
    pub attack_speed: u32,
    pub block_rate: u32,
    pub block_strength: u32,
    pub tenacity: u32,
    pub attack_power: u32,
    pub defense: u32,
    pub direct_hit_rate: u32,
    pub evasion: u32,
    pub magic_defense: u32,
    pub critical_hit_power: u32,
    pub critical_hit_resilience: u32,
    pub critical_hit: u32,
    pub critical_hit_evasion: u32,
    pub slashing_resistance: u32,
    pub piercing_resistance: u32,
    pub blunt_resistance: u32,
    pub projectile_resistance: u32,
    pub attack_magic_potency: u32,
    pub healing_magic_potency: u32,
    pub enhancement_magic_potency: u32,
    pub elemental_bonus: u32,
    pub fire_resistance: u32,
    pub ice_resistance: u32,
    pub wind_resistance: u32,
    pub earth_resistance: u32,
    pub lightning_resistance: u32,
    pub water_resistance: u32,
    pub magic_resistance: u32,
    pub determination: u32,
    pub skill_speed: u32,
    pub spell_speed: u32,
    pub haste: u32,
    pub morale: u32,
    pub enmity: u32,
    pub enmity_reduction: u32,
    pub desynthesis_skill_gain: u32,
    pub exp_bonus: u32,
    pub regen: u32,
    pub special_attribute: u32,
    pub main_attribute: u32,
    pub secondary_attribute: u32,
    pub slow_resistance: u32,
    pub petrification_resistance: u32,
    pub paralysis_resistance: u32,
    pub silence_resistance: u32,
    pub blind_resistance: u32,
    pub posion_resistance: u32,
    pub stun_resistance: u32,
    pub sleep_resistance: u32,
    pub bind_resistance: u32,
    pub heavy_resistance: u32,
    pub doom_resistance: u32,
    pub reduced_durability_loss: u32,
    pub increased_spiritbond_gain: u32,
    pub craftmanship: u32,
    pub control: u32,
    pub gathering: u32,
    pub perception: u32,
}

impl BaseParameters {
    pub fn from_attributes(attributes: &Attributes) -> Self {
        Self {
            strength: attributes.strength,
            dexterity: attributes.dexterity,
            vitality: attributes.vitality,
            intelligence: attributes.intelligence,
            mind: attributes.mind,
            ..Default::default()
        }
    }

    pub fn get_mut(&mut self, index: u8) -> &mut u32 {
        match index {
            1 => &mut self.strength,
            2 => &mut self.dexterity,
            3 => &mut self.vitality,
            4 => &mut self.intelligence,
            5 => &mut self.mind,
            6 => &mut self.piety,
            7 => &mut self.hp,
            8 => &mut self.mp,
            9 => &mut self.tp,
            10 => &mut self.gp,
            11 => &mut self.cp,
            12 => &mut self.physical_damage,
            13 => &mut self.magic_damage,
            14 => &mut self.delay,
            15 => &mut self.additional_effect,
            16 => &mut self.attack_speed,
            17 => &mut self.block_rate,
            18 => &mut self.block_strength,
            19 => &mut self.tenacity,
            20 => &mut self.attack_power,
            21 => &mut self.defense,
            22 => &mut self.direct_hit_rate,
            23 => &mut self.evasion,
            24 => &mut self.magic_defense,
            25 => &mut self.critical_hit_power,
            26 => &mut self.critical_hit_resilience,
            27 => &mut self.critical_hit,
            28 => &mut self.critical_hit_evasion,
            29 => &mut self.slashing_resistance,
            30 => &mut self.piercing_resistance,
            31 => &mut self.blunt_resistance,
            32 => &mut self.projectile_resistance,
            33 => &mut self.attack_magic_potency,
            34 => &mut self.healing_magic_potency,
            35 => &mut self.enhancement_magic_potency,
            36 => &mut self.elemental_bonus,
            37 => &mut self.fire_resistance,
            38 => &mut self.ice_resistance,
            39 => &mut self.wind_resistance,
            40 => &mut self.earth_resistance,
            41 => &mut self.lightning_resistance,
            42 => &mut self.water_resistance,
            43 => &mut self.magic_resistance,
            44 => &mut self.determination,
            45 => &mut self.skill_speed,
            46 => &mut self.spell_speed,
            47 => &mut self.haste,
            48 => &mut self.morale,
            49 => &mut self.enmity,
            50 => &mut self.enmity_reduction,
            51 => &mut self.desynthesis_skill_gain,
            52 => &mut self.exp_bonus,
            53 => &mut self.regen,
            54 => &mut self.special_attribute,
            55 => &mut self.main_attribute,
            56 => &mut self.secondary_attribute,
            57 => &mut self.slow_resistance,
            58 => &mut self.petrification_resistance,
            59 => &mut self.paralysis_resistance,
            60 => &mut self.silence_resistance,
            61 => &mut self.blind_resistance,
            62 => &mut self.posion_resistance,
            63 => &mut self.stun_resistance,
            64 => &mut self.sleep_resistance,
            65 => &mut self.bind_resistance,
            66 => &mut self.heavy_resistance,
            67 => &mut self.doom_resistance,
            68 => &mut self.reduced_durability_loss,
            69 => &mut self.increased_spiritbond_gain,
            70 => &mut self.craftmanship,
            71 => &mut self.control,
            72 => &mut self.gathering,
            73 => &mut self.perception,
            _ => unreachable!(),
        }
    }

    pub fn calculate_hp_mp(&mut self, param_grow: &ParamGrow) {
        self.hp = param_grow.hp_modifier as u32
            + ((self.vitality - BASE_STAT as u32) as f32 * 20.25).round() as u32;
        self.mp = param_grow.mp_modifier as u32;
    }
}

impl ZoneConnection {
    pub async fn update_class_info(&mut self) {
        let ipc;
        {
            let game_data = self.gamedata.lock();

            ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateClassInfo(UpdateClassInfo {
                class_id: self.player_data.classjob_id,
                class_level: self.current_level(&game_data),
                current_level: self.current_level(&game_data),
                current_exp: self.current_exp(&game_data),
                ..Default::default()
            }));
        }
        self.send_ipc_self(ipc).await;

        // Update rested EXP so the bar doesn't reset.
        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::UpdateRestedExp {
                exp: self.player_data.rested_exp as u32,
            },
        })
        .await;
    }

    fn calculate_stat_across_all_items(&self, base_params: &mut BaseParameters) {
        let mut gamedata = self.gamedata.lock();

        for i in 0..self.player_data.inventory.equipped.max_slots() {
            let slot = self.player_data.inventory.equipped.get_slot(i as u16);
            if slot.quantity > 0 {
                let item_info = gamedata
                    .get_item_info(crate::ItemInfoQuery::ById(slot.id))
                    .unwrap();
                for (i, param_id) in item_info.base_param_ids.iter().enumerate() {
                    if *param_id != 0 {
                        *base_params.get_mut(*param_id) += item_info.base_param_values[i] as u32; // TODO: is there ever negative values?
                    }
                }
            }
        }
    }

    // TODO: use for materia melds
    fn _get_equip_slot_percent(param: &BaseParam, equip_category: u8) -> u16 {
        match equip_category {
            1 => param.one_hand_weapon_percent,
            2 => param.off_hand_percent,
            3 => param.head_percent,
            4 => param.chest_percent,
            5 => param.hands_percent,
            6 => param.waist_percent,
            7 => param.legs_percent,
            8 => param.feet_percent,
            9 => param.earring_percent,
            10 => param.necklace_percent,
            11 => param.bracelet_percent,
            12 => param.ring_percent,
            13 => param.two_hand_weapon_percent,
            14 => param.under_armor_percent,
            _ => unreachable!(),
        }
    }

    pub fn base_parameters(&self) -> BaseParameters {
        let attributes;
        let param_grow;

        {
            let mut game_data = self.gamedata.lock();

            attributes = game_data
                .get_racial_base_attributes(self.player_data.subrace)
                .expect("Failed to read racial attributes");

            let level = self.current_level(&game_data);

            param_grow = game_data
                .get_param_grow(level as u32)
                .expect("Failed to read param grow");
        }

        let mut base_parameters = BaseParameters::from_attributes(&attributes);
        self.calculate_stat_across_all_items(&mut base_parameters);
        base_parameters.calculate_hp_mp(&param_grow);

        base_parameters
    }

    pub async fn send_stats(&mut self) {
        let base_parameters = self.base_parameters();
        let attributes;
        {
            let mut game_data = self.gamedata.lock();

            attributes = game_data
                .get_racial_base_attributes(self.player_data.subrace)
                .expect("Failed to read racial attributes");
        }

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PlayerStats(PlayerStats {
            strength: base_parameters.strength,
            dexterity: base_parameters.dexterity,
            vitality: base_parameters.vitality,
            intelligence: base_parameters.intelligence,
            mind: base_parameters.mind,
            piety: base_parameters.piety,
            hp: base_parameters.hp,
            mp: base_parameters.mp,
            tp: base_parameters.tp,
            gp: base_parameters.gp,
            cp: base_parameters.cp,
            delay: base_parameters.delay,
            tenacity: base_parameters.tenacity,
            attack_power: base_parameters.attack_power,
            defense: base_parameters.defense,
            direct_hit_rate: base_parameters.direct_hit_rate,
            evasion: base_parameters.evasion,
            magic_defense: base_parameters.magic_defense,
            critical_hit: base_parameters.critical_hit,
            attack_magic_potency: base_parameters.attack_magic_potency,
            healing_magic_potency: base_parameters.healing_magic_potency,
            elemental_bonus: base_parameters.elemental_bonus,
            determination: base_parameters.determination,
            skill_speed: base_parameters.skill_speed,
            spell_speed: base_parameters.spell_speed,
            haste: base_parameters.haste,
            craftmanship: base_parameters.craftmanship,
            control: base_parameters.control,
            gathering: base_parameters.gathering,
            perception: base_parameters.perception,
            base_strength: attributes.strength,
            base_dexterity: attributes.dexterity,
            base_vitality: attributes.vitality,
            base_intelligence: attributes.intelligence,
            base_mind: attributes.mind,
            base_piety: attributes.piety,
        }));
        self.send_ipc_self(ipc).await;
    }

    /// Inform the server of new updated level/HP/MP stats.
    pub async fn update_server_stats(&mut self) {
        let current_level;
        {
            let gamedata = self.gamedata.lock();
            current_level = self.current_level(&gamedata);
        }

        let base_parameters = self.base_parameters();

        self.handle
            .send(ToServer::SetNewStatValues(
                self.player_data.actor_id,
                current_level as u8,
                self.player_data.classjob_id,
                base_parameters.hp,
                base_parameters.mp as u16,
            ))
            .await;
    }

    pub fn current_level(&self, game_data: &GameData) -> u16 {
        let index = game_data
            .get_exp_array_index(self.player_data.classjob_id as u16)
            .unwrap();
        self.player_data.classjob_levels.0[index as usize]
    }

    pub fn set_current_level(&mut self, level: u16) {
        self.set_level_for(self.player_data.classjob_id, level);
    }

    pub fn set_level_for(&mut self, classjob_id: u8, level: u16) {
        let game_data = self.gamedata.lock();

        let index = game_data.get_exp_array_index(classjob_id as u16).unwrap();
        self.player_data.classjob_levels.0[index as usize] = level;
    }

    pub fn current_exp(&self, game_data: &GameData) -> i32 {
        let index = game_data
            .get_exp_array_index(self.player_data.classjob_id as u16)
            .unwrap();
        self.player_data.classjob_exp.0[index as usize]
    }

    pub fn set_current_exp(&mut self, exp: i32) {
        let game_data = self.gamedata.lock();

        let index = game_data
            .get_exp_array_index(self.player_data.classjob_id as u16)
            .unwrap();
        self.player_data.classjob_exp.0[index as usize] = exp;
    }

    pub async fn update_hp_mp(&mut self, actor_id: ObjectId, hp: u32, mp: u16) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateHpMpTp { hp, mp, unk: 0 });

        self.send_segment(PacketSegment {
            source_actor: actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        })
        .await;
    }

    /// Adds EXP to the current classjob, handles level-up and so on.
    pub async fn add_exp(&mut self, exp: i32) {
        let (bonus_percent, exp) = self.use_exp_bonus(exp);

        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::EXPFloatingMessage {
                classjob_id: self.player_data.classjob_id as u32,
                amount: exp as u32,
                bonus_percent: bonus_percent as u32,
            },
        })
        .await;

        self.send_rested_exp().await; // If the EXP bonus was used, we need to update in case.

        let index;
        let mut level_up = 0;
        {
            let mut game_data = self.gamedata.lock();

            index = game_data
                .get_exp_array_index(self.player_data.classjob_id as u16)
                .unwrap();

            self.player_data.classjob_exp.0[index as usize] += exp;

            // Keep going until we have leftover EXP
            loop {
                let curr_exp = self.player_data.classjob_exp.0[index as usize];
                let max_exp = game_data
                    .get_max_exp(self.player_data.classjob_levels.0[index as usize] as u32);
                let difference = curr_exp - max_exp;
                if difference >= 0 {
                    level_up += 1;
                    self.player_data.classjob_exp.0[index as usize] = difference;
                } else {
                    break;
                }
            }
        }

        if level_up > 0 {
            let curr_level = self.player_data.classjob_levels.0[index as usize];
            let new_level = curr_level + level_up;
            self.set_current_level(new_level);

            self.actor_control_self(ActorControlSelf {
                category: ActorControlCategory::LevelUpMessage {
                    classjob_id: self.player_data.classjob_id as u32,
                    level: new_level as u32,
                    unk2: 0,
                    unk3: 0,
                },
            })
            .await;
        }

        self.send_stats().await;
        self.update_class_info().await;
    }

    /// The number of seconds to add to the rested EXP bonus.
    pub async fn add_rested_exp_seconds(&mut self, seconds: i32) {
        self.player_data.rested_exp += seconds;
        self.player_data.rested_exp = self.player_data.rested_exp.clamp(0, MAXIMUM_RESTED_EXP);

        self.send_rested_exp().await;
    }

    /// Sends the rested EXP bonus to the client.
    pub async fn send_rested_exp(&mut self) {
        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::UpdateRestedExp {
                exp: self.player_data.rested_exp as u32,
            },
        })
        .await;
    }

    /// "Use" an EXP bonus for the specified amount. Returns the bonus percentage and new amount of EXP earned.
    /// Remember to update rested EXP when calling this function!
    pub fn use_exp_bonus(&mut self, exp: i32) -> (i32, i32) {
        let mut bonus_percent = 0;

        // TODO: Please write a unit test for this
        if self.player_data.rested_exp > 0 {
            // Here is where the fun calculations come in for rested EXP.
            // We need to basically convert EXP to "seconds" - which is what rested EXP is counted in.

            let mut gamedata = self.gamedata.lock();
            let current_level = self.current_level(&gamedata);

            // This is the size of the bar in EXP.
            let max_exp = gamedata.get_max_exp(current_level as u32);
            assert!(max_exp > 0);

            // This is the size of the bar in seconds.
            let max_seconds = 201600;

            // Get a relative amount of the bar.
            let new_exp_relative = exp as f32 / max_exp as f32;

            // Get the amount of seconds to remove from the rested EXP bonus.
            let seconds_to_remove = new_exp_relative * max_seconds as f32;
            self.player_data.rested_exp -= seconds_to_remove.round() as i32;

            // Add that sweet EXP bonus.
            bonus_percent += 50;
        }

        // Add EXP bonus on top of already earned EXP.
        let exp = exp + (exp * (bonus_percent as f32 / 100.0).round() as i32);

        (bonus_percent, exp)
    }
}
