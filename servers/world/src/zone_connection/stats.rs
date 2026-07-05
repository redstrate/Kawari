//! Managing statistics, including your classjob and other related information.

use crate::{
    GameData, ToServer, ZoneConnection,
    gamedata::{Attributes, Modifiers},
    inventory::{EquippedStorage, Storage},
};
use icarus::ParamGrow::ParamGrowRow;
use kawari::{
    common::{MAXIMUM_RESTED_EXP, ObjectId},
    ipc::zone::{
        ActorControlCategory, DamageKind, PlayerStats, ServerZoneIpcData, ServerZoneIpcSegment,
        UpdateClassInfo,
    },
};
use mlua::{UserData, UserDataMethods};

#[derive(Clone, Copy)]
struct LevelModifier {
    main: u16,
    sub: u16,
    div: u16,
}

const LEVEL_MODIFIERS: [LevelModifier; 101] = [
    LevelModifier {
        main: 20,
        sub: 56,
        div: 56,
    },
    LevelModifier {
        main: 20,
        sub: 56,
        div: 56,
    },
    LevelModifier {
        main: 21,
        sub: 57,
        div: 57,
    },
    LevelModifier {
        main: 22,
        sub: 60,
        div: 60,
    },
    LevelModifier {
        main: 24,
        sub: 62,
        div: 62,
    },
    LevelModifier {
        main: 26,
        sub: 65,
        div: 65,
    },
    LevelModifier {
        main: 27,
        sub: 68,
        div: 68,
    },
    LevelModifier {
        main: 29,
        sub: 70,
        div: 70,
    },
    LevelModifier {
        main: 31,
        sub: 73,
        div: 73,
    },
    LevelModifier {
        main: 33,
        sub: 76,
        div: 76,
    },
    LevelModifier {
        main: 35,
        sub: 78,
        div: 78,
    },
    LevelModifier {
        main: 36,
        sub: 82,
        div: 82,
    },
    LevelModifier {
        main: 38,
        sub: 85,
        div: 85,
    },
    LevelModifier {
        main: 41,
        sub: 89,
        div: 89,
    },
    LevelModifier {
        main: 44,
        sub: 93,
        div: 93,
    },
    LevelModifier {
        main: 46,
        sub: 96,
        div: 96,
    },
    LevelModifier {
        main: 49,
        sub: 100,
        div: 100,
    },
    LevelModifier {
        main: 52,
        sub: 104,
        div: 104,
    },
    LevelModifier {
        main: 54,
        sub: 109,
        div: 109,
    },
    LevelModifier {
        main: 57,
        sub: 113,
        div: 113,
    },
    LevelModifier {
        main: 60,
        sub: 116,
        div: 116,
    },
    LevelModifier {
        main: 63,
        sub: 122,
        div: 122,
    },
    LevelModifier {
        main: 67,
        sub: 127,
        div: 127,
    },
    LevelModifier {
        main: 71,
        sub: 133,
        div: 133,
    },
    LevelModifier {
        main: 74,
        sub: 138,
        div: 138,
    },
    LevelModifier {
        main: 78,
        sub: 144,
        div: 144,
    },
    LevelModifier {
        main: 81,
        sub: 150,
        div: 150,
    },
    LevelModifier {
        main: 85,
        sub: 155,
        div: 155,
    },
    LevelModifier {
        main: 89,
        sub: 162,
        div: 162,
    },
    LevelModifier {
        main: 92,
        sub: 168,
        div: 168,
    },
    LevelModifier {
        main: 97,
        sub: 173,
        div: 173,
    },
    LevelModifier {
        main: 101,
        sub: 181,
        div: 181,
    },
    LevelModifier {
        main: 106,
        sub: 188,
        div: 188,
    },
    LevelModifier {
        main: 110,
        sub: 194,
        div: 194,
    },
    LevelModifier {
        main: 115,
        sub: 202,
        div: 202,
    },
    LevelModifier {
        main: 119,
        sub: 209,
        div: 209,
    },
    LevelModifier {
        main: 124,
        sub: 215,
        div: 215,
    },
    LevelModifier {
        main: 128,
        sub: 223,
        div: 223,
    },
    LevelModifier {
        main: 134,
        sub: 229,
        div: 229,
    },
    LevelModifier {
        main: 139,
        sub: 236,
        div: 236,
    },
    LevelModifier {
        main: 144,
        sub: 244,
        div: 244,
    },
    LevelModifier {
        main: 150,
        sub: 253,
        div: 253,
    },
    LevelModifier {
        main: 155,
        sub: 263,
        div: 263,
    },
    LevelModifier {
        main: 161,
        sub: 272,
        div: 272,
    },
    LevelModifier {
        main: 166,
        sub: 283,
        div: 283,
    },
    LevelModifier {
        main: 171,
        sub: 292,
        div: 292,
    },
    LevelModifier {
        main: 177,
        sub: 302,
        div: 302,
    },
    LevelModifier {
        main: 183,
        sub: 311,
        div: 311,
    },
    LevelModifier {
        main: 189,
        sub: 322,
        div: 322,
    },
    LevelModifier {
        main: 196,
        sub: 331,
        div: 331,
    },
    LevelModifier {
        main: 202,
        sub: 341,
        div: 341,
    },
    LevelModifier {
        main: 204,
        sub: 342,
        div: 366,
    },
    LevelModifier {
        main: 205,
        sub: 344,
        div: 392,
    },
    LevelModifier {
        main: 207,
        sub: 345,
        div: 418,
    },
    LevelModifier {
        main: 209,
        sub: 346,
        div: 444,
    },
    LevelModifier {
        main: 210,
        sub: 347,
        div: 470,
    },
    LevelModifier {
        main: 212,
        sub: 349,
        div: 496,
    },
    LevelModifier {
        main: 214,
        sub: 350,
        div: 522,
    },
    LevelModifier {
        main: 215,
        sub: 351,
        div: 548,
    },
    LevelModifier {
        main: 217,
        sub: 352,
        div: 574,
    },
    LevelModifier {
        main: 218,
        sub: 354,
        div: 600,
    },
    LevelModifier {
        main: 224,
        sub: 355,
        div: 630,
    },
    LevelModifier {
        main: 228,
        sub: 356,
        div: 660,
    },
    LevelModifier {
        main: 236,
        sub: 357,
        div: 690,
    },
    LevelModifier {
        main: 244,
        sub: 358,
        div: 720,
    },
    LevelModifier {
        main: 252,
        sub: 359,
        div: 750,
    },
    LevelModifier {
        main: 260,
        sub: 360,
        div: 780,
    },
    LevelModifier {
        main: 268,
        sub: 361,
        div: 810,
    },
    LevelModifier {
        main: 276,
        sub: 362,
        div: 840,
    },
    LevelModifier {
        main: 284,
        sub: 363,
        div: 870,
    },
    LevelModifier {
        main: 292,
        sub: 364,
        div: 900,
    },
    LevelModifier {
        main: 296,
        sub: 365,
        div: 940,
    },
    LevelModifier {
        main: 300,
        sub: 366,
        div: 980,
    },
    LevelModifier {
        main: 305,
        sub: 367,
        div: 1020,
    },
    LevelModifier {
        main: 310,
        sub: 368,
        div: 1060,
    },
    LevelModifier {
        main: 315,
        sub: 370,
        div: 1100,
    },
    LevelModifier {
        main: 320,
        sub: 372,
        div: 1140,
    },
    LevelModifier {
        main: 325,
        sub: 374,
        div: 1180,
    },
    LevelModifier {
        main: 330,
        sub: 376,
        div: 1220,
    },
    LevelModifier {
        main: 335,
        sub: 378,
        div: 1260,
    },
    LevelModifier {
        main: 340,
        sub: 380,
        div: 1300,
    },
    LevelModifier {
        main: 345,
        sub: 382,
        div: 1360,
    },
    LevelModifier {
        main: 350,
        sub: 384,
        div: 1420,
    },
    LevelModifier {
        main: 355,
        sub: 386,
        div: 1480,
    },
    LevelModifier {
        main: 360,
        sub: 388,
        div: 1540,
    },
    LevelModifier {
        main: 365,
        sub: 390,
        div: 1600,
    },
    LevelModifier {
        main: 370,
        sub: 392,
        div: 1660,
    },
    LevelModifier {
        main: 375,
        sub: 394,
        div: 1720,
    },
    LevelModifier {
        main: 380,
        sub: 396,
        div: 1780,
    },
    LevelModifier {
        main: 385,
        sub: 398,
        div: 1840,
    },
    LevelModifier {
        main: 390,
        sub: 400,
        div: 1900,
    },
    LevelModifier {
        main: 395,
        sub: 402,
        div: 1988,
    },
    LevelModifier {
        main: 400,
        sub: 404,
        div: 2076,
    },
    LevelModifier {
        main: 405,
        sub: 406,
        div: 2164,
    },
    LevelModifier {
        main: 410,
        sub: 408,
        div: 2252,
    },
    LevelModifier {
        main: 415,
        sub: 410,
        div: 2340,
    },
    LevelModifier {
        main: 420,
        sub: 412,
        div: 2428,
    },
    LevelModifier {
        main: 425,
        sub: 414,
        div: 2516,
    },
    LevelModifier {
        main: 430,
        sub: 416,
        div: 2604,
    },
    LevelModifier {
        main: 435,
        sub: 418,
        div: 2692,
    },
    LevelModifier {
        main: 440,
        sub: 420,
        div: 2780,
    },
];

fn level_modifier_for(level: u32) -> LevelModifier {
    LEVEL_MODIFIERS[level.clamp(1, 100) as usize]
}

fn attack_modifier_for_level(level: u32) -> f64 {
    match level {
        0..=50 => 75.0,
        51..=70 => ((level - 50) as f64 * 2.5) + 75.0,
        71..=80 => ((level - 70) as f64 * 4.0) + 125.0,
        81..=90 => ((level - 80) as f64 * 3.0) + 165.0,
        _ => ((level - 90) as f64 * 4.2) + 195.0,
    }
}

fn heal_modifier_for_level(level: u32) -> f64 {
    match level {
        0..=59 => (level as f64 * 1.5) + 10.0,
        60..=69 => ((level - 60) as f64 * 2.0) + 100.0,
        70..=79 => 120.0,
        _ => ((level - 80) as f64 * 2.5) + 120.8,
    }
}

fn tank_attack_modifier_for_level(level: u32) -> f64 {
    match level {
        0..=80 => level as f64 + 35.0,
        81..=90 => ((level - 80) as f64 * 4.1) + 115.0,
        _ => ((level - 90) as f64 * 3.4) + 156.0,
    }
}

fn positive_scaled_bonus(value: u32, baseline: u16, coefficient: f64, divisor: u16) -> u32 {
    let baseline = u32::from(baseline);
    if value <= baseline || divisor == 0 {
        return 0;
    }

    (coefficient * (value - baseline) as f64 / f64::from(divisor)).floor() as u32
}

fn apply_factor(value: u64, factor: u32, divisor: u64) -> u64 {
    value.saturating_mul(u64::from(factor)) / divisor
}

fn classjob_uses_dexterity(classjob_id: u8) -> bool {
    matches!(classjob_id, 5 | 23 | 29 | 30 | 31 | 38 | 41)
}

fn classjob_uses_mind(classjob_id: u8) -> bool {
    matches!(classjob_id, 6 | 24 | 28 | 33 | 40)
}

fn classjob_uses_tenacity(classjob_id: u8) -> bool {
    matches!(classjob_id, 1 | 3 | 19 | 21 | 32 | 37)
}

fn classjob_is_caster_like(classjob_id: u8) -> bool {
    matches!(
        classjob_id,
        6 | 7 | 24 | 25 | 26 | 27 | 28 | 33 | 35 | 36 | 40 | 42
    )
}

fn classjob_primary_stat_id(classjob_id: u8) -> u8 {
    if classjob_uses_mind(classjob_id) {
        5
    } else if classjob_is_caster_like(classjob_id) {
        4
    } else if classjob_uses_dexterity(classjob_id) {
        2
    } else {
        1
    }
}

fn classjob_damage_trait_modifier(classjob_id: u8, level: u32) -> f64 {
    if classjob_id == 36 {
        return match level {
            50.. => 1.5,
            40..=49 => 1.4,
            30..=39 => 1.3,
            20..=29 => 1.2,
            10..=19 => 1.1,
            _ => 1.0,
        };
    }

    if classjob_is_caster_like(classjob_id) {
        return match level {
            40.. => 1.3,
            20..=39 => 1.1,
            _ => 1.0,
        };
    }

    match classjob_id {
        5 | 23 | 31 => match level {
            40.. => 1.2,
            20..=39 => 1.1,
            _ => 1.0,
        },
        38 => match level {
            60.. => 1.2,
            50..=59 => 1.1,
            _ => 1.0,
        },
        _ => 1.0,
    }
}

/// Every BaseParam row, some of them may be useless.
#[derive(Default, Debug, Clone)]
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
    pub classjob_id: u8,
    pub level: u8,
    pub job_attack_modifier: u16,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DamageRollModifiers {
    pub crit_rate_bonus: f64,
    pub direct_hit_rate_bonus: f64,
    pub force_critical: bool,
    pub force_direct_hit: bool,
}

impl BaseParameters {
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

    /// Calculates a set of attributes based on the level and class modifiers.
    pub fn calculate_based_on_level(
        &mut self,
        attributes: &Attributes,
        level: u32,
        classjob_id: u8,
        param_grow: &ParamGrowRow,
        modifiers: &Modifiers,
    ) {
        self.classjob_id = classjob_id;
        self.level = level.min(u32::from(u8::MAX)) as u8;
        self.job_attack_modifier = match classjob_primary_stat_id(classjob_id) {
            1 => modifiers.strength,
            2 => modifiers.dexterity,
            4 => modifiers.intelligence,
            5 => modifiers.mind,
            _ => 100,
        };

        // Akh Morning data can't be extrapolated from game sheets, and it's missing a significant amount of entries from 50-70, so let's try making a decent approximation with the BaseSpeed, HpModifier and LevelModifier columns.
        self.strength = modifiers
            .apply_to(1, param_grow.BaseSpeed as u32)
            .saturating_add_signed(attributes.strength as i32);
        self.dexterity = modifiers
            .apply_to(2, param_grow.BaseSpeed as u32)
            .saturating_add_signed(attributes.dexterity as i32);
        self.vitality = modifiers
            .apply_to(3, param_grow.BaseSpeed as u32)
            .saturating_add_signed(attributes.vitality as i32);
        self.intelligence = modifiers
            .apply_to(4, param_grow.BaseSpeed as u32)
            .saturating_add_signed(attributes.intelligence as i32);
        self.mind = modifiers
            .apply_to(5, param_grow.BaseSpeed as u32)
            .saturating_add_signed(attributes.mind as i32);
        self.piety = modifiers
            .apply_to(6, param_grow.BaseSpeed as u32)
            .saturating_add_signed(attributes.piety as i32);

        self.spell_speed = param_grow.BaseSpeed as u32;
        self.tenacity = param_grow.BaseSpeed as u32;
        self.skill_speed = self.tenacity;
        self.haste = 100; // Controls cast times

        // This is fixed and isn't modified by any items in retail, so it's safe to be set here.
        self.mp = param_grow.MpModifier as u32;
    }

    // This should be called after item stat calculations.
    pub fn calculate_potencies(
        &mut self,
        _level: u32,
        param_grow: &ParamGrowRow,
        modifiers: Option<&Modifiers>,
    ) {
        let primary_damage_stat = self.primary_damage_stat_value();
        self.attack_power = primary_damage_stat;
        self.attack_magic_potency = if classjob_is_caster_like(self.classjob_id) {
            primary_damage_stat
        } else {
            self.intelligence
        };
        self.healing_magic_potency = if classjob_uses_mind(self.classjob_id) {
            self.mind
        } else {
            primary_damage_stat
        };

        // To calculate HP, we use a formula loosely inspired by Akh Morning and take some liberties to keep it fairly simple, at least for now.
        // TODO: This formula isn't the greatest for 1-50, as near the end of that range it's fairly low compared to retail. For level 80+ though it's pretty close.
        // In clearer terms without all the casts: (100 + hp_modifier + (total_vit - base_vit) * (level_mod / 100)) * job_hp_modifier
        let classjob_hp_mod;
        let classjob_vit_mod;

        if let Some(modifiers) = modifiers {
            classjob_hp_mod = modifiers.hp as f32 / 100.0;
            classjob_vit_mod = modifiers.vitality as f32 / 100.0;
        } else {
            classjob_hp_mod = 1.0;
            classjob_vit_mod = 1.0;
        };

        let hp_mod = param_grow.HpModifier as f32;
        let base_vit = (param_grow.BaseSpeed as f32) * classjob_vit_mod; // TODO: Tribe adjustments, if we care about such a minimal change?
        let lv_mod = param_grow.LevelModifier as f32;

        self.hp = (100.0
            + hp_mod
            + ((self.vitality as f32 - base_vit) * (lv_mod / 100.0)) * classjob_hp_mod)
            .round() as u32;
    }

    fn primary_damage_stat_value(&self) -> u32 {
        match classjob_primary_stat_id(self.classjob_id) {
            2 => self.dexterity,
            4 => self.intelligence,
            5 => self.mind,
            _ => self.strength,
        }
    }

    fn expected_crit_rate(&self, level_modifier: LevelModifier) -> f64 {
        f64::from(
            50 + positive_scaled_bonus(
                self.critical_hit,
                level_modifier.sub,
                200.0,
                level_modifier.div,
            ),
        ) / 1000.0
    }

    fn crit_damage_factor(&self, level_modifier: LevelModifier) -> u32 {
        1400 + positive_scaled_bonus(
            self.critical_hit,
            level_modifier.sub,
            200.0,
            level_modifier.div,
        )
    }

    fn direct_hit_rate_bonus(&self, level_modifier: LevelModifier) -> f64 {
        f64::from(positive_scaled_bonus(
            self.direct_hit_rate,
            level_modifier.sub,
            550.0,
            level_modifier.div,
        )) / 1000.0
    }

    fn offensive_weapon_damage(&self) -> u32 {
        if classjob_is_caster_like(self.classjob_id) {
            self.magic_damage
        } else {
            self.physical_damage
        }
    }

    fn weapon_damage_factor(&self, level_modifier: LevelModifier) -> u32 {
        self.offensive_weapon_damage()
            + (u32::from(level_modifier.main) * u32::from(self.job_attack_modifier) / 1000)
    }

    fn attack_factor(&self, level_modifier: LevelModifier) -> u32 {
        let coefficient = if classjob_uses_tenacity(self.classjob_id) {
            tank_attack_modifier_for_level(u32::from(self.level))
        } else {
            attack_modifier_for_level(u32::from(self.level))
        };
        100 + positive_scaled_bonus(
            self.primary_damage_stat_value(),
            level_modifier.main,
            coefficient,
            level_modifier.main,
        )
    }

    fn heal_factor(&self, level_modifier: LevelModifier) -> u32 {
        100 + positive_scaled_bonus(
            self.primary_damage_stat_value(),
            level_modifier.main,
            heal_modifier_for_level(u32::from(self.level)),
            level_modifier.main,
        )
    }

    fn determination_factor(&self, level_modifier: LevelModifier) -> u32 {
        1000 + positive_scaled_bonus(
            self.determination,
            level_modifier.main,
            140.0,
            level_modifier.div,
        )
    }

    fn tenacity_factor(&self, level_modifier: LevelModifier) -> u32 {
        if !classjob_uses_tenacity(self.classjob_id) {
            return 1000;
        }

        1000 + positive_scaled_bonus(self.tenacity, level_modifier.sub, 112.0, level_modifier.div)
    }

    fn damage_trait_factor(&self) -> u32 {
        (classjob_damage_trait_modifier(self.classjob_id, u32::from(self.level)) * 100.0).round()
            as u32
    }

    fn calc_expected_damage(&self, potency: u32) -> u32 {
        if potency == 0 {
            return 0;
        }

        let level_modifier = level_modifier_for(u32::from(self.level));
        let weapon_damage_factor = self.weapon_damage_factor(level_modifier);
        if self.primary_damage_stat_value() == 0 || weapon_damage_factor == 0 {
            return 0;
        }

        let mut damage = u64::from(potency);
        damage = apply_factor(damage, self.attack_factor(level_modifier), 100);
        damage = apply_factor(damage, weapon_damage_factor, 100);
        damage = apply_factor(damage, self.determination_factor(level_modifier), 1000);
        damage = apply_factor(damage, self.tenacity_factor(level_modifier), 1000);
        damage = apply_factor(damage, self.damage_trait_factor(), 100);

        // Return the *base* (pre-crit, pre-direct-hit, pre-variance) damage. The crit/direct-hit
        // roll and ±5% variance are applied later by `roll_damage` at the point of impact, so the
        // client can be told the actual hit severity (and so each hit varies).
        damage.min(u64::from(u32::MAX)) as u32
    }

    /// Rolls a single damage instance from a base amount: independently rolls critical hit and
    /// direct hit from this character's rates, applies the ±5% damage variance, and reports which
    /// (if any) occurred so the client can show the right hit severity.
    pub fn roll_damage(&self, base: u32) -> (u32, DamageKind) {
        self.roll_damage_with_modifiers(base, DamageRollModifiers::default())
    }

    /// Rolls a single damage instance, allowing action/status effects to add crit or direct-hit
    /// chance, or force either roll.
    pub fn roll_damage_with_modifiers(
        &self,
        base: u32,
        modifiers: DamageRollModifiers,
    ) -> (u32, DamageKind) {
        if base == 0 {
            return (0, DamageKind::Normal);
        }

        let level_modifier = level_modifier_for(u32::from(self.level));
        let crit_rate =
            (self.expected_crit_rate(level_modifier) + modifiers.crit_rate_bonus).clamp(0.0, 1.0);
        let direct_hit_rate = (self.direct_hit_rate_bonus(level_modifier)
            + modifiers.direct_hit_rate_bonus)
            .clamp(0.0, 1.0);
        let is_crit = modifiers.force_critical || fastrand::f64() < crit_rate;
        let is_direct = modifiers.force_direct_hit || fastrand::f64() < direct_hit_rate;

        let mut damage = u64::from(base);
        if is_crit {
            damage = apply_factor(damage, self.crit_damage_factor(level_modifier), 1000);
        }
        if is_direct {
            damage = apply_factor(damage, 125, 100);
        }
        // Retail damage variance is a whole-percent roll from 95% through 105%.
        damage = apply_factor(damage, 95 + fastrand::u32(0..11), 100);

        let kind = match (is_crit, is_direct) {
            (true, true) => DamageKind::CriticalDirectHit,
            (true, false) => DamageKind::Critical,
            (false, true) => DamageKind::DirectHit,
            (false, false) => DamageKind::Normal,
        };

        (damage.min(u64::from(u32::MAX)) as u32, kind)
    }

    /// Fraction of incoming damage (0.0–0.99) mitigated by this character's defense, per the
    /// retail formula (15% at a defense equal to the level's divisor). `is_magic` selects magic
    /// defense over physical defense.
    pub fn mitigation_against(&self, is_magic: bool) -> f64 {
        let level_modifier = level_modifier_for(u32::from(self.level));
        let defense = if is_magic {
            self.magic_defense
        } else {
            self.defense
        };
        (0.15 * defense as f64 / level_modifier.div as f64).clamp(0.0, 0.99)
    }

    /// Applies this character's skill/spell speed to a base time, matching the client's exact
    /// rounding (CharacterPanelRefined's SpeedCalc). Input and output are both in centiseconds
    /// (10ms units) — the same unit the client's cooldown packets use — so both sides agree to the
    /// centisecond and the GCD ring doesn't rubber-band. Casters use spell speed, others skill speed.
    pub fn apply_speed(&self, base_centisec: u32) -> u32 {
        if base_centisec == 0 {
            return 0;
        }
        let speed = if classjob_is_caster_like(self.classjob_id) {
            self.spell_speed
        } else {
            self.skill_speed
        };
        let level_modifier = level_modifier_for(u32::from(self.level));
        // factor = 1000 + ceil(130*(sub - speed)/div) = 1000 - floor(130*(speed - sub)/div).
        let factor = 1000.0
            - (130.0 * (speed as f64 - level_modifier.sub as f64) / level_modifier.div as f64)
                .floor();
        // Client formula (haste/type modifier assumed 0): floor(floor(factor * base / 100) / 10).
        let inner = (factor * base_centisec as f64 / 100.0).floor();
        ((inner / 10.0).floor() as u32).max(1)
    }

    /// Calculates amount of physical damage to apply based on potency.
    pub fn calc_physical_damage(&self, potency: u32) -> u32 {
        self.calc_expected_damage(potency)
    }

    /// Calculates amount of magic damage to apply based on potency.
    pub fn calc_magical_damage(&self, potency: u32) -> u32 {
        self.calc_expected_damage(potency)
    }

    /// Calculates amount of healing to apply based on potency.
    pub fn calc_heal_amount(&self, potency: u32) -> u32 {
        if potency == 0 {
            return 0;
        }

        let level_modifier = level_modifier_for(u32::from(self.level));
        let weapon_damage_factor = self.weapon_damage_factor(level_modifier);
        if self.primary_damage_stat_value() == 0 || weapon_damage_factor == 0 {
            return 0;
        }

        let mut heal = u64::from(potency);
        heal = apply_factor(heal, self.heal_factor(level_modifier), 100);
        heal = apply_factor(heal, weapon_damage_factor, 100);
        heal = apply_factor(heal, self.determination_factor(level_modifier), 1000);
        heal = apply_factor(heal, self.tenacity_factor(level_modifier), 1000);
        if classjob_is_caster_like(self.classjob_id) {
            heal = apply_factor(heal, self.damage_trait_factor(), 100);
        }
        // Roll a critical heal (heals can crit but never direct-hit) plus ±5% variance.
        let mut heal = heal;
        if fastrand::f64() < self.expected_crit_rate(level_modifier) {
            heal = apply_factor(heal, self.crit_damage_factor(level_modifier), 1000);
        }
        heal = apply_factor(heal, 95 + fastrand::u32(0..11), 100);

        heal.min(u64::from(u32::MAX)) as u32
    }

    /// Iterates over the given equipped items and calculates defense, along with any stat bonuses.
    pub fn calculate_stat_across_all_items(
        &mut self,
        equipped: &EquippedStorage,
        item_level_attributes: Option<&[u16]>,
    ) {
        for i in 0..equipped.max_slots() {
            let slot = equipped.get_slot(i as u16);
            if slot.quantity > 0 {
                self.defense += slot.defense as u32;
                self.magic_defense += slot.magic_defense as u32;
                // Weapon base damage drives calc_physical/magical_damage; only the equipped
                // weapon carries a non-zero value, so summing across slots is fine.
                self.physical_damage += slot.weapon_damage_phys as u32;
                self.magic_damage += slot.weapon_damage_mag as u32;

                for (i, param_id) in slot.base_param_ids.iter().enumerate() {
                    if *param_id != 0 {
                        // Make sure to cap attributes when ilvl syncing:
                        let value = if let Some(item_level_attributes) = item_level_attributes {
                            let attribute_cap = item_level_attributes[i];
                            (slot.base_param_values[i].min(attribute_cap as i16)) as u32 // TODO: is there ever negative values?
                        } else {
                            slot.base_param_values[i] as u32 // TODO: is there ever negative values?
                        };
                        *self.get_mut(*param_id) += value;
                    }
                }
            }
        }
    }
}

impl UserData for BaseParameters {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("calc_physical_damage", |_, this, potency: u32| {
            Ok(this.calc_physical_damage(potency))
        });
        methods.add_method("calc_magical_damage", |_, this, potency: u32| {
            Ok(this.calc_magical_damage(potency))
        });
        methods.add_method("calc_heal_amount", |_, this, potency: u32| {
            Ok(this.calc_heal_amount(potency))
        });
        methods.add_method("max_hp", |_, this, _: ()| Ok(this.hp));
    }
}

impl ZoneConnection {
    pub async fn update_class_info(&mut self) {
        let ipc;
        {
            let game_data = self.gamedata.lock();

            ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateClassInfo(UpdateClassInfo {
                class_id: self.player_data.classjob.current_class as u8,
                class_level: self.current_level(&game_data),
                current_level: self.current_level(&game_data),
                synced_level: self.synced_level.unwrap_or_default() as u16,
                current_exp: self.current_exp(&game_data),
                ..Default::default()
            }));
        }
        self.send_ipc_self(ipc).await;

        // Update rested EXP so the bar doesn't reset.
        self.actor_control_self(ActorControlCategory::UpdateRestedExp {
            exp: self.player_data.classjob.rested_exp as u32,
        })
        .await;

        // Send this too, otherwise actions dependent on the gauge won't function
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorGauge {
            classjob_id: self.player_data.classjob.current_class as u8,
            data: 0,
        });
        self.send_ipc_self(ipc).await;
    }

    pub async fn finish_changing_class(&mut self) {
        // Play the VFX!
        self.broadcast_actor_control(ActorControlCategory::ClassJobChangeVFX {
            classjob_id: self.player_data.classjob.current_class as u32,
        })
        .await;

        let ipc;
        {
            let game_data = self.gamedata.lock();

            ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UnkClassRelated {
                classjob_id: self.player_data.classjob.current_class as u8,
                class_level: self.current_level(&game_data),
                current_level: self.current_level(&game_data),
            });
        }
        self.send_ipc_self(ipc).await;

        // Commit back our classjob data after changing. Done here so it gets picked up by any paths that change classjob.
        {
            let mut db = self.database.lock();
            db.commit_classjob_and_inventory(&self.player_data);
        }
    }

    /// Scaled parameters based on level and item level sync.
    pub fn base_parameters(&self) -> BaseParameters {
        let mut game_data = self.gamedata.lock();

        let modifiers = game_data
            .get_class_job_modifiers(self.player_data.classjob.current_class as u32)
            .expect("Failed to read param grow");

        let attributes = game_data
            .get_racial_base_attributes(self.player_data.subrace)
            .expect("Failed to read racial attributes");

        let level = self
            .synced_level
            .map(|x| x as u16)
            .unwrap_or(self.current_level(&game_data));

        let item_level_sync;
        {
            let param_grow = game_data
                .get_param_grow(level as u32)
                .expect("Failed to read param grow");

            if self.synced_level.is_some() {
                item_level_sync = Some(param_grow.ItemLevelSync);
            } else {
                item_level_sync = None;
            }
        }

        let item_level_attributes =
            game_data.get_item_level_attributes(item_level_sync.unwrap_or_default());

        let param_grow = game_data
            .get_param_grow(level as u32)
            .expect("Failed to read param grow");

        let mut base_parameters = BaseParameters::default();
        base_parameters.calculate_based_on_level(
            &attributes,
            level as u32,
            self.player_data.classjob.current_class as u8,
            &param_grow,
            &modifiers,
        );
        base_parameters.calculate_stat_across_all_items(
            &self.player_data.inventory.equipped,
            if item_level_sync.is_some() {
                Some(&item_level_attributes)
            } else {
                None
            },
        );
        base_parameters.calculate_potencies(level as u32, &param_grow, Some(&modifiers));

        base_parameters
    }

    /// Same as `base_parameters` but doesn't take into account level or item level sync.
    pub fn unscaled_base_parameters(&mut self) -> BaseParameters {
        let mut game_data = self.gamedata.lock();

        let modifiers = game_data
            .get_class_job_modifiers(self.player_data.classjob.current_class as u32)
            .expect("Failed to read param grow");

        let attributes = game_data
            .get_racial_base_attributes(self.player_data.subrace)
            .expect("Failed to read racial attributes");

        let level = self.current_level(&game_data);

        let param_grow = game_data
            .get_param_grow(level as u32)
            .expect("Failed to read param grow");

        let mut base_parameters = BaseParameters::default();
        base_parameters.calculate_based_on_level(
            &attributes,
            level as u32,
            self.player_data.classjob.current_class as u8,
            &param_grow,
            &modifiers,
        );
        base_parameters.calculate_stat_across_all_items(&self.player_data.inventory.equipped, None);
        base_parameters.calculate_potencies(level as u32, &param_grow, Some(&modifiers));

        base_parameters
    }

    pub async fn send_stats(&mut self) {
        let base_parameters = self.base_parameters();
        let unscaled_base_parameters = self.unscaled_base_parameters();

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
            base_strength: unscaled_base_parameters.strength,
            base_dexterity: unscaled_base_parameters.dexterity,
            base_vitality: unscaled_base_parameters.vitality,
            base_intelligence: unscaled_base_parameters.intelligence,
            base_mind: unscaled_base_parameters.mind,
            base_piety: unscaled_base_parameters.piety,
        }));
        self.send_ipc_self(ipc).await;

        self.update_server_stats().await;
    }

    /// Inform the server of new updated level/HP/MP stats.
    async fn update_server_stats(&mut self) {
        let current_level;
        {
            let gamedata = self.gamedata.lock();
            current_level = self.current_level(&gamedata);
        }

        let base_parameters = self.base_parameters();

        self.handle
            .send(ToServer::SetNewStatValues(
                self.player_data.character.actor_id,
                current_level as u8,
                self.player_data.classjob.current_class as u8,
                base_parameters,
            ))
            .await;
    }

    pub fn current_level(&self, game_data: &GameData) -> u16 {
        let index = game_data
            .get_exp_array_index(self.player_data.classjob.current_class as u16)
            .expect("Failed to find EXP array index?!");
        self.player_data.classjob.levels.0[index as usize]
    }

    pub fn set_current_level(&mut self, level: u16) {
        self.set_level_for(self.player_data.classjob.current_class as u8, level);
    }

    pub fn set_level_for(&mut self, classjob_id: u8, level: u16) {
        let game_data = self.gamedata.lock();

        let index = game_data
            .get_exp_array_index(classjob_id as u16)
            .expect("Failed to find EXP array index?!");
        self.player_data.classjob.levels.0[index as usize] = level;
    }

    /// Returns the current level stored in the EXP array slot for the given classjob.
    /// Note: some classes/jobs share an EXP array slot (e.g. ACN/SMN/SCH all use index 18),
    /// so this reflects the shared level for those.
    pub fn level_for(&self, classjob_id: u8) -> u16 {
        let game_data = self.gamedata.lock();

        let index = game_data
            .get_exp_array_index(classjob_id as u16)
            .expect("Failed to find EXP array index?!");
        self.player_data.classjob.levels.0[index as usize]
    }

    pub fn current_exp(&self, game_data: &GameData) -> i32 {
        let index = game_data
            .get_exp_array_index(self.player_data.classjob.current_class as u16)
            .expect("Failed to find EXP array index?!");
        self.player_data.classjob.exp.0[index as usize]
    }

    pub fn set_current_exp(&mut self, exp: i32) {
        let game_data = self.gamedata.lock();

        let index = game_data
            .get_exp_array_index(self.player_data.classjob.current_class as u16)
            .expect("Failed to find EXP array index?!");
        self.player_data.classjob.exp.0[index as usize] = exp;
    }

    pub async fn update_hp_mp(&mut self, actor_id: ObjectId, hp: u32, mp: u16) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateHpMpTp { hp, mp, unk: 0 });

        self.send_ipc_from(actor_id, ipc).await;
    }

    /// Adds EXP to the current classjob, handles level-up and so on.
    pub async fn add_exp(&mut self, exp: i32) {
        let (bonus_percent, exp) = self.use_exp_bonus(exp);

        self.actor_control_self(ActorControlCategory::EXPFloatingMessage {
            classjob_id: self.player_data.classjob.current_class as u32,
            amount: exp as u32,
            bonus_percent: bonus_percent as u32,
        })
        .await;

        self.send_rested_exp().await; // If the EXP bonus was used, we need to update in case.

        let index;
        let mut level_up = 0;
        {
            let mut game_data = self.gamedata.lock();

            index = game_data
                .get_exp_array_index(self.player_data.classjob.current_class as u16)
                .expect("Failed to find EXP array index?!");

            self.player_data.classjob.exp.0[index as usize] += exp;

            // Keep going until we have leftover EXP
            loop {
                let curr_exp = self.player_data.classjob.exp.0[index as usize];
                let max_exp = game_data
                    .get_max_exp(self.player_data.classjob.levels.0[index as usize] as u32);
                let difference = curr_exp - max_exp;
                if difference >= 0 {
                    level_up += 1;
                    self.player_data.classjob.exp.0[index as usize] = difference;
                } else {
                    break;
                }
            }
        }

        if level_up > 0 {
            let curr_level = self.player_data.classjob.levels.0[index as usize];
            let new_level = curr_level + level_up;
            self.set_current_level(new_level);

            self.actor_control_self(ActorControlCategory::LevelUpMessage {
                classjob_id: self.player_data.classjob.current_class as u32,
                level: new_level as u32,
                unk2: 0,
                unk3: 0,
            })
            .await;
        }

        self.send_stats().await;
        self.update_class_info().await;
    }

    /// The number of seconds to add to the rested EXP bonus.
    pub async fn add_rested_exp_seconds(&mut self, seconds: i32) {
        self.player_data.classjob.rested_exp += seconds;
        self.player_data.classjob.rested_exp = self
            .player_data
            .classjob
            .rested_exp
            .clamp(0, MAXIMUM_RESTED_EXP);

        self.send_rested_exp().await;
    }

    /// Sends the rested EXP bonus to the client.
    pub async fn send_rested_exp(&mut self) {
        self.actor_control_self(ActorControlCategory::UpdateRestedExp {
            exp: self.player_data.classjob.rested_exp as u32,
        })
        .await;
    }

    /// "Use" an EXP bonus for the specified amount. Returns the bonus percentage and new amount of EXP earned.
    /// Remember to update rested EXP when calling this function!
    pub fn use_exp_bonus(&mut self, exp: i32) -> (i32, i32) {
        let mut bonus_percent = 0;

        // TODO: Please write a unit test for this
        if self.player_data.classjob.rested_exp > 0 {
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
            self.player_data.classjob.rested_exp -= seconds_to_remove.round() as i32;

            // Add that sweet EXP bonus.
            bonus_percent += 50;
        }

        // Add EXP bonus on top of already earned EXP.
        let exp = exp + (exp * (bonus_percent as f32 / 100.0).round() as i32);

        (bonus_percent, exp)
    }
}
