//! Bard (BRD) job-specific logic: action remaps, gauge state, and status syncing.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::{
    StatusEffects,
    gamedata::GameData,
    server::{actor::NetworkedActor, combat_state::PlayerCombatState},
};
use kawari::{common::ObjectId, ipc::zone::ActionRequest};

/// ClassJob row id for Bard (BRD). NOTE: 5 is Archer (ARC); Bard is 23.
const CLASSJOB_BARD: u8 = 23;
/// Some data paths carry the Bard ClassJobCategory row instead of the ClassJob row.
const CLASSJOB_CATEGORY_BARD: u8 = 24;

// ==================== Action IDs ====================

// GCD Actions
const ACTION_HEAVY_SHOT: u32 = 97;
const ACTION_BURST_SHOT: u32 = 16495;
const ACTION_REFULGENT_ARROW: u32 = 7409;
const ACTION_SHADOWBITE: u32 = 16494;
const ACTION_CAUSTIC_BITE: u32 = 7406;
const ACTION_STORMBITE: u32 = 7407;
const ACTION_IRON_JAWS: u32 = 3560;
const ACTION_APEX_ARROW: u32 = 16496;
const ACTION_BLAST_ARROW: u32 = 25784;

// Song Actions
const ACTION_MAGES_BALLAD: u32 = 114;
const ACTION_ARMYS_PAEON: u32 = 116;
const ACTION_WANDERERS_MINUET: u32 = 3559;

// Ability Actions (oGCD)
const ACTION_EMPYREAL_ARROW: u32 = 3558;
const ACTION_SIDEWINDER: u32 = 3562;
const ACTION_PITCH_PERFECT: u32 = 7404;
const ACTION_RADIANT_FINALE: u32 = 25785;
const ACTION_BATTLE_VOICE: u32 = 118;
const ACTION_RAGING_STRIKES: u32 = 101;
const ACTION_BARRAGE: u32 = 107;
const ACTION_TROUBADOUR: u32 = 7405;
const ACTION_RAIN_OF_DEATH: u32 = 117;
const ACTION_NATURES_MINNE: u32 = 7408;
const ACTION_WARDENS_PAEAN: u32 = 3561;
const ACTION_REPELLING_SHOT: u32 = 112;
const ACTION_HEARTBREAK_SHOT: u32 = 36975;
const ACTION_RESONANT_ARROW: u32 = 36976;
const ACTION_RADIANT_ENCORE: u32 = 36977;

// ==================== Status IDs ====================

// DoT Statuses
const STATUS_CAUSTIC_BITE: u16 = 1200;
const STATUS_STORMBITE: u16 = 1201;

// Song Statuses
const STATUS_MAGES_BALLAD: u16 = 2217;
const STATUS_ARMYS_PAEON: u16 = 2218;
const STATUS_WANDERERS_MINUET: u16 = 2216;

// Song Proc Statuses
const STATUS_ARMYS_MUSE: u16 = 1932; // Army's Paeon proc - skill speed up
const STATUS_ARMYS_ETHOS: u16 = 1933; // Army's Paeon next song proc
const STATUS_REPERTOIRE: u16 = 3137; // Wanderer's Minuet proc - Pitch Perfect ready

// Buff Statuses
const STATUS_RAGING_STRIKES: u16 = 125;
const STATUS_BATTLE_VOICE: u16 = 141;
const STATUS_RADIANT_FINALE: u16 = 2964;
const STATUS_BARRAGE: u16 = 128;
const STATUS_TROUBADOUR: u16 = 1934;
const STATUS_NATURES_MINNE: u16 = 1202;

// Ready Statuses
const STATUS_HAWK_EYE: u16 = 3861; // Refulgent Arrow / Shadowbite ready
const STATUS_SHADOWBITE_READY: u16 = 3002;
const STATUS_BLAST_ARROW_READY: u16 = 2692;
const STATUS_BLAST_ARROW_READY_2: u16 = 3142; // Alternative status ID
const STATUS_RESONANT_ARROW_READY: u16 = 3862;
const STATUS_RADIANT_ENCORE_READY: u16 = 3863;

// ==================== Gauge Constants ====================

const BARD_GAUGE_SOUL_VOICE: u8 = 0;
const MAX_SOUL_VOICE: u8 = 100;
const REPERTOIRE_PROC_CHANCE_PERCENT: u8 = 80;
const REPERTOIRE_SOUL_VOICE_GAIN: u8 = 5;
const WANDERERS_REPERTOIRE_MAX: u8 = 3;
const ARMYS_REPERTOIRE_MAX: u8 = 4;
const BLOODLETTER_COOLDOWN_GROUP_INDEX: usize = 9;

const SONG_FLAG_MAGES_BALLAD: u8 = 1 << 0;
const SONG_FLAG_ARMYS_PAEON: u8 = 1 << 1;
const SONG_FLAG_WANDERERS_MINUET: u8 = SONG_FLAG_MAGES_BALLAD | SONG_FLAG_ARMYS_PAEON;
const SONG_FLAG_MAGES_BALLAD_LAST_PLAYED: u8 = 1 << 2;
const SONG_FLAG_ARMYS_PAEON_LAST_PLAYED: u8 = 1 << 3;
const SONG_FLAG_WANDERERS_MINUET_LAST_PLAYED: u8 =
    SONG_FLAG_MAGES_BALLAD_LAST_PLAYED | SONG_FLAG_ARMYS_PAEON_LAST_PLAYED;
const SONG_FLAG_MAGES_BALLAD_CODA: u8 = 1 << 4;
const SONG_FLAG_ARMYS_PAEON_CODA: u8 = 1 << 5;
const SONG_FLAG_WANDERERS_MINUET_CODA: u8 = 1 << 6;
const SONG_FLAG_ACTIVE_MASK: u8 = SONG_FLAG_MAGES_BALLAD | SONG_FLAG_ARMYS_PAEON;
const SONG_FLAG_LAST_PLAYED_MASK: u8 =
    SONG_FLAG_MAGES_BALLAD_LAST_PLAYED | SONG_FLAG_ARMYS_PAEON_LAST_PLAYED;
const SONG_FLAG_CODA_MASK: u8 =
    SONG_FLAG_MAGES_BALLAD_CODA | SONG_FLAG_ARMYS_PAEON_CODA | SONG_FLAG_WANDERERS_MINUET_CODA;

const LEVEL_RADIANT_FINALE_CODA: u8 = 90;
const LEVEL_REPERTOIRE_SOUL_VOICE: u8 = 80;
const LEVEL_EMPYREAL_ARROW_REPERTOIRE: u8 = 68;

// ==================== Duration Constants ====================

const DOT_DURATION: Duration = Duration::from_secs(45);
const SONG_DURATION: Duration = Duration::from_secs(45);
const REPERTOIRE_TICK_INTERVAL: Duration = Duration::from_secs(3);
const MAGES_BALLAD_COOLDOWN_REDUCTION: Duration = Duration::from_millis(7500);
const RAGING_STRIKES_DURATION: Duration = Duration::from_secs(20);
const BATTLE_VOICE_DURATION: Duration = Duration::from_secs(15);
const RADIANT_FINALE_DURATION: Duration = Duration::from_secs(20);
const BARRAGE_DURATION: Duration = Duration::from_secs(10);
const TROUBADOUR_DURATION: Duration = Duration::from_secs(15);
const NATURES_MINNE_DURATION: Duration = Duration::from_secs(15);
const READY_STATUS_DURATION: Duration = Duration::from_secs(30);
const HEAVY_SHOT_HAWK_EYE_PROC_CHANCE_PERCENT: u8 = 20;
const BURST_SHOT_HAWK_EYE_PROC_CHANCE_PERCENT: u8 = 35;

/// Bard's current song
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BardSong {
    #[default]
    None,
    MagesBallad,
    ArmysPaeon,
    WanderersMinuet,
}

/// Bard job state tracked server-side
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BardState {
    /// Current Soul Voice gauge (0-100)
    pub soul_voice: u8,
    /// Current Repertoire stacks for Army's Paeon / Wanderer's Minuet.
    #[serde(default)]
    pub repertoire: u8,
    /// Currently active song
    pub current_song: BardSong,
    /// SongFlags byte consumed by the client BardGauge.
    #[serde(default)]
    pub song_flags: u8,
    /// When the current song expires
    #[serde(skip)]
    pub song_expires_at: Option<Instant>,
    /// Next 3-second song tick that can roll Repertoire while in combat.
    #[serde(skip)]
    pub next_repertoire_tick_at: Option<Instant>,
    /// Caustic Bite DoT expiration
    #[serde(skip)]
    pub caustic_bite_expires_at: Option<Instant>,
    /// Stormbite DoT expiration
    #[serde(skip)]
    pub stormbite_expires_at: Option<Instant>,
    /// Hawk Eye stacks (0-3) for Refulgent Arrow ready
    pub hawk_eye_stacks: u8,
    /// When Hawk Eye expires
    #[serde(skip)]
    pub hawk_eye_expires_at: Option<Instant>,
    /// Blast Arrow ready status
    pub blast_arrow_ready: bool,
    /// Resonant Arrow ready status
    pub resonant_arrow_ready: bool,
    /// Radiant Encore ready status
    pub radiant_encore_ready: bool,
    /// Damage bonus granted by the current Radiant Finale status: 2/4/6.
    #[serde(default)]
    pub radiant_finale_damage_bonus_percent: u8,
    /// Barrage stacks remaining
    pub barrage_stacks: u8,
    /// When Barrage expires
    #[serde(skip)]
    pub barrage_expires_at: Option<Instant>,
    /// When Raging Strikes expires
    #[serde(skip)]
    pub raging_strikes_expires_at: Option<Instant>,
    /// When Battle Voice expires
    #[serde(skip)]
    pub battle_voice_expires_at: Option<Instant>,
    /// When Radiant Finale expires
    #[serde(skip)]
    pub radiant_finale_expires_at: Option<Instant>,
    /// When Troubadour expires
    #[serde(skip)]
    pub troubadour_expires_at: Option<Instant>,
    /// When Nature's Minne expires
    #[serde(skip)]
    pub natures_minne_expires_at: Option<Instant>,
}

impl BardState {
    /// Check if any DoT is active
    pub fn has_dot_active(&self) -> bool {
        self.caustic_bite_expires_at
            .is_some_and(|t| t > Instant::now())
            || self
                .stormbite_expires_at
                .is_some_and(|t| t > Instant::now())
    }

    /// Check if a song is currently active
    pub fn has_song_active(&self) -> bool {
        self.current_song != BardSong::None
            && self.song_expires_at.is_some_and(|t| t > Instant::now())
    }

    /// Check if Barrage is active
    pub fn has_barrage_active(&self) -> bool {
        self.barrage_stacks > 0 && self.barrage_expires_at.is_some_and(|t| t > Instant::now())
    }

    /// Get remaining song duration in milliseconds
    pub fn song_remaining_ms(&self) -> u16 {
        self.song_expires_at
            .map(|t| {
                t.saturating_duration_since(Instant::now())
                    .as_millis()
                    .min(u128::from(u16::MAX)) as u16
            })
            .unwrap_or(0)
    }
}

/// Check if the given class_job is Bard
pub(crate) fn is_bard(class_job: u8) -> bool {
    class_job == CLASSJOB_BARD || class_job == CLASSJOB_CATEGORY_BARD
}

pub(crate) fn gauge_class_job_id() -> u8 {
    CLASSJOB_BARD
}

/// Refresh runtime state, clearing expired timers
fn refresh_bard_runtime_state(brd: &mut BardState) {
    let now = Instant::now();

    // Clear expired song
    if brd.song_expires_at.is_some_and(|t| t <= now) {
        brd.current_song = BardSong::None;
        brd.song_expires_at = None;
        brd.next_repertoire_tick_at = None;
        brd.repertoire = 0;
        brd.song_flags &= !SONG_FLAG_ACTIVE_MASK;
    }

    // Clear expired DoTs
    if brd.caustic_bite_expires_at.is_some_and(|t| t <= now) {
        brd.caustic_bite_expires_at = None;
    }
    if brd.stormbite_expires_at.is_some_and(|t| t <= now) {
        brd.stormbite_expires_at = None;
    }

    // Clear expired buffs
    if brd.barrage_expires_at.is_some_and(|t| t <= now) {
        brd.barrage_stacks = 0;
        brd.barrage_expires_at = None;
    }
    if brd.hawk_eye_expires_at.is_some_and(|t| t <= now) {
        brd.hawk_eye_stacks = 0;
        brd.hawk_eye_expires_at = None;
    }
    if brd.raging_strikes_expires_at.is_some_and(|t| t <= now) {
        brd.raging_strikes_expires_at = None;
    }
    if brd.battle_voice_expires_at.is_some_and(|t| t <= now) {
        brd.battle_voice_expires_at = None;
    }
    if brd.radiant_finale_expires_at.is_some_and(|t| t <= now) {
        brd.radiant_finale_expires_at = None;
        brd.radiant_finale_damage_bonus_percent = 0;
    }
    if brd.troubadour_expires_at.is_some_and(|t| t <= now) {
        brd.troubadour_expires_at = None;
    }
    if brd.natures_minne_expires_at.is_some_and(|t| t <= now) {
        brd.natures_minne_expires_at = None;
    }
}

fn song_active_flags(song: BardSong) -> u8 {
    match song {
        BardSong::None => 0,
        BardSong::MagesBallad => SONG_FLAG_MAGES_BALLAD,
        BardSong::ArmysPaeon => SONG_FLAG_ARMYS_PAEON,
        BardSong::WanderersMinuet => SONG_FLAG_WANDERERS_MINUET,
    }
}

fn song_last_played_flags(song: BardSong) -> u8 {
    match song {
        BardSong::None => 0,
        BardSong::MagesBallad => SONG_FLAG_MAGES_BALLAD_LAST_PLAYED,
        BardSong::ArmysPaeon => SONG_FLAG_ARMYS_PAEON_LAST_PLAYED,
        BardSong::WanderersMinuet => SONG_FLAG_WANDERERS_MINUET_LAST_PLAYED,
    }
}

fn song_coda_flags(song: BardSong) -> u8 {
    match song {
        BardSong::None => 0,
        BardSong::MagesBallad => SONG_FLAG_MAGES_BALLAD_CODA,
        BardSong::ArmysPaeon => SONG_FLAG_ARMYS_PAEON_CODA,
        BardSong::WanderersMinuet => SONG_FLAG_WANDERERS_MINUET_CODA,
    }
}

fn coda_count(song_flags: u8) -> u8 {
    (song_flags & SONG_FLAG_CODA_MASK).count_ones() as u8
}

fn start_song(brd: &mut BardState, song: BardSong, level: u8) {
    let now = Instant::now();
    brd.current_song = song;
    brd.song_expires_at = Some(now + SONG_DURATION);
    brd.next_repertoire_tick_at = Some(now + REPERTOIRE_TICK_INTERVAL);
    brd.repertoire = 0;
    brd.song_flags &= !(SONG_FLAG_ACTIVE_MASK | SONG_FLAG_LAST_PLAYED_MASK);
    brd.song_flags |= song_active_flags(song) | song_last_played_flags(song);
    if level >= LEVEL_RADIANT_FINALE_CODA {
        brd.song_flags |= song_coda_flags(song);
    }
    brd.soul_voice = (brd.soul_voice + 20).min(MAX_SOUL_VOICE);
}

fn take_due_repertoire_tick(brd: &mut BardState) -> bool {
    if !brd.has_song_active() {
        brd.next_repertoire_tick_at = None;
        return false;
    }

    let now = Instant::now();
    let Some(next_tick) = brd.next_repertoire_tick_at else {
        brd.next_repertoire_tick_at = Some(now + REPERTOIRE_TICK_INTERVAL);
        return false;
    };

    if next_tick > now {
        return false;
    }

    brd.next_repertoire_tick_at = Some(now + REPERTOIRE_TICK_INTERVAL);
    true
}

fn song_remaining_secs(brd: &BardState) -> f32 {
    brd.song_expires_at
        .map(|t| t.saturating_duration_since(Instant::now()).as_secs_f32())
        .unwrap_or(0.0)
}

#[derive(Debug, Default, Clone, Copy)]
struct BardRepertoireProc {
    changed: bool,
    reduce_bloodletter_cooldown: bool,
    status_param: Option<u16>,
}

fn grant_repertoire(brd: &mut BardState, level: u8) -> BardRepertoireProc {
    if !brd.has_song_active() {
        return BardRepertoireProc::default();
    }

    let before_soul_voice = brd.soul_voice;
    let before_repertoire = brd.repertoire;
    let mut reduce_bloodletter_cooldown = false;
    let status_param = match brd.current_song {
        BardSong::MagesBallad => {
            reduce_bloodletter_cooldown = true;
            Some(0)
        }
        BardSong::ArmysPaeon => {
            brd.repertoire = brd.repertoire.saturating_add(1).min(ARMYS_REPERTOIRE_MAX);
            Some(u16::from(brd.repertoire))
        }
        BardSong::WanderersMinuet => {
            brd.repertoire = brd
                .repertoire
                .saturating_add(1)
                .min(WANDERERS_REPERTOIRE_MAX);
            Some(u16::from(brd.repertoire))
        }
        BardSong::None => None,
    };

    if level >= LEVEL_REPERTOIRE_SOUL_VOICE {
        brd.soul_voice = brd
            .soul_voice
            .saturating_add(REPERTOIRE_SOUL_VOICE_GAIN)
            .min(MAX_SOUL_VOICE);
    }

    BardRepertoireProc {
        changed: brd.soul_voice != before_soul_voice
            || brd.repertoire != before_repertoire
            || reduce_bloodletter_cooldown,
        reduce_bloodletter_cooldown,
        status_param,
    }
}

fn add_repertoire_status(
    status_effects: &mut StatusEffects,
    brd: &BardState,
    owner_actor_id: ObjectId,
    status_param: u16,
) -> bool {
    let duration = song_remaining_secs(brd).max(0.1);
    status_effects.add_with_source(STATUS_REPERTOIRE, status_param, duration, owner_actor_id);
    true
}

fn remove_repertoire_status(status_effects: &mut StatusEffects) -> bool {
    if status_effects.get(STATUS_REPERTOIRE).is_none() {
        return false;
    }

    status_effects.remove(STATUS_REPERTOIRE);
    true
}

fn add_hawk_eye_status(
    status_effects: &mut StatusEffects,
    brd: &mut BardState,
    owner_actor_id: ObjectId,
    stacks: u8,
) -> bool {
    brd.hawk_eye_stacks = stacks;
    brd.hawk_eye_expires_at = Some(Instant::now() + READY_STATUS_DURATION);
    status_effects.add_with_source(
        STATUS_HAWK_EYE,
        0,
        READY_STATUS_DURATION.as_secs_f32(),
        owner_actor_id,
    );
    true
}

fn remove_hawk_eye_status(status_effects: &mut StatusEffects, brd: &mut BardState) -> bool {
    brd.hawk_eye_stacks = 0;
    brd.hawk_eye_expires_at = None;
    if status_effects.get(STATUS_HAWK_EYE).is_none() {
        return false;
    }

    status_effects.remove(STATUS_HAWK_EYE);
    true
}

fn remove_status_if_present(status_effects: &mut StatusEffects, status_id: u16) -> bool {
    if status_effects.get(status_id).is_none() {
        return false;
    }

    status_effects.remove(status_id);
    true
}

fn maybe_proc_hawk_eye(
    status_effects: &mut StatusEffects,
    brd: &mut BardState,
    owner_actor_id: ObjectId,
    chance_percent: u8,
) -> bool {
    if fastrand::u8(0..100) >= chance_percent {
        return false;
    }

    add_hawk_eye_status(status_effects, brd, owner_actor_id, 1)
}

fn apply_repertoire_proc(
    combat_state: &mut PlayerCombatState,
    status_effects: &mut StatusEffects,
    owner_actor_id: ObjectId,
    level: u8,
) -> BardActionUpdate {
    let proc = grant_repertoire(&mut combat_state.bard, level);
    let mut status_timer_refreshed = false;
    if let Some(status_param) = proc.status_param {
        status_timer_refreshed = add_repertoire_status(
            status_effects,
            &combat_state.bard,
            owner_actor_id,
            status_param,
        );
    }

    let cooldown_update = if proc.reduce_bloodletter_cooldown {
        combat_state
            .reduce_cooldown_recovery(
                BLOODLETTER_COOLDOWN_GROUP_INDEX,
                MAGES_BALLAD_COOLDOWN_REDUCTION,
            )
            .map(|(elapsed_centisec, total_centisec)| BardCooldownUpdate {
                cooldown_group: BLOODLETTER_COOLDOWN_GROUP_INDEX as u32,
                elapsed_centisec,
                total_centisec,
            })
    } else {
        None
    };

    BardActionUpdate {
        changed: proc.changed || status_timer_refreshed || cooldown_update.is_some(),
        status_timer_refreshed,
        cooldown_update,
    }
}

fn visible_song_flags(brd: &BardState) -> u8 {
    let mut song_flags = brd.song_flags;
    song_flags &= !SONG_FLAG_ACTIVE_MASK;
    if brd.has_song_active() {
        song_flags |= song_active_flags(brd.current_song);
    }
    song_flags
}

/// Resolve Bard action remapping (skill morphing)
pub(crate) fn resolve_bard_action(
    request: &ActionRequest,
    combat_state: &PlayerCombatState,
    level: u8,
    _game_data: &mut GameData,
) -> u32 {
    let mut brd = combat_state.bard.clone();
    refresh_bard_runtime_state(&mut brd);

    match request.action_id {
        // Burst Shot → Refulgent Arrow when Hawk Eye is active (level 70+)
        ACTION_BURST_SHOT | ACTION_HEAVY_SHOT if brd.hawk_eye_stacks > 0 && level >= 70 => {
            tracing::debug!("Burst Shot -> Refulgent Arrow (Hawk Eye active)");
            ACTION_REFULGENT_ARROW
        }

        // Shadowbite requires Hawk Eye or Shadowbite Ready status
        ACTION_SHADOWBITE if brd.hawk_eye_stacks > 0 => {
            tracing::debug!("Shadowbite allowed (Hawk Eye active)");
            ACTION_SHADOWBITE
        }

        // Apex Arrow → Blast Arrow when Blast Arrow Ready
        ACTION_APEX_ARROW if brd.blast_arrow_ready => {
            tracing::debug!("Apex Arrow -> Blast Arrow (Blast Arrow Ready)");
            ACTION_BLAST_ARROW
        }

        // Barrage button becomes Resonant Arrow while the Dawntrail follow-up is ready.
        ACTION_BARRAGE if brd.resonant_arrow_ready && level >= 96 => ACTION_RESONANT_ARROW,

        // Radiant Finale button becomes Radiant Encore while the follow-up is ready.
        ACTION_RADIANT_FINALE if brd.radiant_encore_ready && level >= 100 => ACTION_RADIANT_ENCORE,

        // Default: no remapping
        _ => request.action_id,
    }
}

/// Check if the Bard can execute the given action
pub(crate) fn can_execute_bard_action(
    action_id: u32,
    combat_state: &PlayerCombatState,
    level: u8,
) -> bool {
    let mut brd = combat_state.bard.clone();
    refresh_bard_runtime_state(&mut brd);

    match action_id {
        // Songs require level check
        ACTION_MAGES_BALLAD => level >= 30,
        ACTION_ARMYS_PAEON => level >= 40,
        ACTION_WANDERERS_MINUET => level >= 52,

        // Pitch Perfect requires Wanderer's Minuet active
        ACTION_PITCH_PERFECT => {
            brd.current_song == BardSong::WanderersMinuet
                && brd.has_song_active()
                && brd.repertoire > 0
        }

        // Apex Arrow requires Soul Voice >= 80
        ACTION_APEX_ARROW => brd.soul_voice >= 80 && level >= 80,

        // Blast Arrow requires Blast Arrow Ready
        ACTION_BLAST_ARROW => brd.blast_arrow_ready,

        // Iron Jaws requires at least one DoT active
        ACTION_IRON_JAWS => brd.has_dot_active(),

        // Heartbreak Shot requires level 92
        ACTION_HEARTBREAK_SHOT => level >= 92,

        // Resonant Arrow requires Resonant Arrow Ready
        ACTION_RESONANT_ARROW => brd.resonant_arrow_ready,

        // Radiant Encore requires Radiant Encore Ready
        ACTION_RADIANT_ENCORE => brd.radiant_encore_ready,

        // Default: allow execution
        _ => true,
    }
}

/// Update Bard state after an action is executed
pub(crate) fn update_bard_state_after_action(
    action_id: u32,
    actor: &mut NetworkedActor,
    owner_actor_id: ObjectId,
) -> BardActionUpdate {
    let level = actor.get_common_spawn().level;
    let NetworkedActor::Player {
        combat_state,
        status_effects,
        ..
    } = actor
    else {
        return BardActionUpdate::default();
    };

    refresh_bard_runtime_state(&mut combat_state.bard);
    let mut action_update = BardActionUpdate::default();

    match action_id {
        // Song activations
        ACTION_MAGES_BALLAD => {
            start_song(&mut combat_state.bard, BardSong::MagesBallad, level);
            action_update.status_timer_refreshed = remove_repertoire_status(status_effects);
        }
        ACTION_ARMYS_PAEON => {
            start_song(&mut combat_state.bard, BardSong::ArmysPaeon, level);
            action_update.status_timer_refreshed = remove_repertoire_status(status_effects);
        }
        ACTION_WANDERERS_MINUET => {
            start_song(&mut combat_state.bard, BardSong::WanderersMinuet, level);
            action_update.status_timer_refreshed = remove_repertoire_status(status_effects);
        }

        // DoT applications
        ACTION_CAUSTIC_BITE => {
            combat_state.bard.caustic_bite_expires_at = Some(Instant::now() + DOT_DURATION);
        }
        ACTION_STORMBITE => {
            combat_state.bard.stormbite_expires_at = Some(Instant::now() + DOT_DURATION);
        }

        // Iron Jaws refreshes both DoTs
        ACTION_IRON_JAWS => {
            if combat_state.bard.caustic_bite_expires_at.is_some() {
                combat_state.bard.caustic_bite_expires_at = Some(Instant::now() + DOT_DURATION);
            }
            if combat_state.bard.stormbite_expires_at.is_some() {
                combat_state.bard.stormbite_expires_at = Some(Instant::now() + DOT_DURATION);
            }
            combat_state.bard.soul_voice = (combat_state.bard.soul_voice + 10).min(MAX_SOUL_VOICE);
        }

        // Apex Arrow consumes Soul Voice
        ACTION_APEX_ARROW => {
            combat_state.bard.soul_voice = 0;
            if level >= 86 {
                combat_state.bard.blast_arrow_ready = true;
            }
        }

        // Blast Arrow consumes ready status
        ACTION_BLAST_ARROW => {
            combat_state.bard.blast_arrow_ready = false;
            action_update.status_timer_refreshed |=
                remove_status_if_present(status_effects, STATUS_BLAST_ARROW_READY);
            action_update.status_timer_refreshed |=
                remove_status_if_present(status_effects, STATUS_BLAST_ARROW_READY_2);
        }

        // GCD weaponskills add Soul Voice and can grant Hawk Eye.
        ACTION_HEAVY_SHOT => {
            combat_state.bard.soul_voice = (combat_state.bard.soul_voice + 5).min(MAX_SOUL_VOICE);
            action_update.status_timer_refreshed |= maybe_proc_hawk_eye(
                status_effects,
                &mut combat_state.bard,
                owner_actor_id,
                HEAVY_SHOT_HAWK_EYE_PROC_CHANCE_PERCENT,
            );
        }
        ACTION_BURST_SHOT => {
            combat_state.bard.soul_voice = (combat_state.bard.soul_voice + 5).min(MAX_SOUL_VOICE);
            action_update.status_timer_refreshed |= maybe_proc_hawk_eye(
                status_effects,
                &mut combat_state.bard,
                owner_actor_id,
                BURST_SHOT_HAWK_EYE_PROC_CHANCE_PERCENT,
            );
        }
        ACTION_REFULGENT_ARROW => {
            combat_state.bard.soul_voice = (combat_state.bard.soul_voice + 10).min(MAX_SOUL_VOICE);
            action_update.status_timer_refreshed |=
                remove_hawk_eye_status(status_effects, &mut combat_state.bard);
            combat_state.bard.resonant_arrow_ready = false;
            action_update.status_timer_refreshed |=
                remove_status_if_present(status_effects, STATUS_RESONANT_ARROW_READY);
        }
        ACTION_SHADOWBITE => {
            combat_state.bard.soul_voice = (combat_state.bard.soul_voice + 10).min(MAX_SOUL_VOICE);
            action_update.status_timer_refreshed |=
                remove_hawk_eye_status(status_effects, &mut combat_state.bard);
            action_update.status_timer_refreshed |=
                remove_status_if_present(status_effects, STATUS_SHADOWBITE_READY);
        }

        ACTION_EMPYREAL_ARROW => {
            if level >= LEVEL_EMPYREAL_ARROW_REPERTOIRE && combat_state.bard.has_song_active() {
                action_update =
                    apply_repertoire_proc(combat_state, status_effects, owner_actor_id, level);
            }
        }

        // Buff activations
        ACTION_RAGING_STRIKES => {
            combat_state.bard.raging_strikes_expires_at =
                Some(Instant::now() + RAGING_STRIKES_DURATION);
        }
        ACTION_BATTLE_VOICE => {
            combat_state.bard.battle_voice_expires_at =
                Some(Instant::now() + BATTLE_VOICE_DURATION);
        }
        ACTION_RADIANT_FINALE => {
            let coda = coda_count(combat_state.bard.song_flags);
            combat_state.bard.radiant_finale_damage_bonus_percent = match coda {
                0 => 0,
                1 => 2,
                2 => 4,
                _ => 6,
            };
            combat_state.bard.radiant_finale_expires_at =
                Some(Instant::now() + RADIANT_FINALE_DURATION);
            combat_state.bard.song_flags &= !SONG_FLAG_CODA_MASK;
            if level >= 100 {
                combat_state.bard.radiant_encore_ready = true;
            }
        }
        ACTION_BARRAGE => {
            combat_state.bard.barrage_stacks = 3;
            combat_state.bard.barrage_expires_at = Some(Instant::now() + BARRAGE_DURATION);
            action_update.status_timer_refreshed |=
                add_hawk_eye_status(status_effects, &mut combat_state.bard, owner_actor_id, 3);
            if level >= 96 {
                combat_state.bard.resonant_arrow_ready = true;
            }
        }
        ACTION_TROUBADOUR => {
            combat_state.bard.troubadour_expires_at = Some(Instant::now() + TROUBADOUR_DURATION);
        }
        ACTION_NATURES_MINNE => {
            combat_state.bard.natures_minne_expires_at =
                Some(Instant::now() + NATURES_MINNE_DURATION);
        }

        // Consume ready statuses
        ACTION_PITCH_PERFECT => {
            combat_state.bard.repertoire = 0;
            action_update.status_timer_refreshed = remove_repertoire_status(status_effects);
        }
        ACTION_RESONANT_ARROW => {
            combat_state.bard.resonant_arrow_ready = false;
            action_update.status_timer_refreshed |=
                remove_status_if_present(status_effects, STATUS_RESONANT_ARROW_READY);
        }
        ACTION_RADIANT_ENCORE => {
            combat_state.bard.radiant_encore_ready = false;
            action_update.status_timer_refreshed |=
                remove_status_if_present(status_effects, STATUS_RADIANT_ENCORE_READY);
        }

        _ => {}
    }

    action_update
}

/// Build gauge data to send to the client.
///
/// The ActorGauge packet carries the 8-byte class-specific tail of the client's BardGauge:
/// bytes 0-1 SongTimer | bytes 2-3 unused | byte 4 Repertoire | byte 5 SoulVoice |
/// byte 6 RadiantFinaleCoda | byte 7 SongFlags.
pub(crate) fn build_bard_gauge_data(combat_state: &PlayerCombatState, _level: u8) -> u64 {
    let mut brd = combat_state.bard.clone();
    refresh_bard_runtime_state(&mut brd);

    let song_remaining = brd.song_remaining_ms();
    let soul_voice = brd.soul_voice;
    let repertoire = match brd.current_song {
        BardSong::ArmysPaeon if brd.has_song_active() => brd.repertoire.min(ARMYS_REPERTOIRE_MAX),
        BardSong::WanderersMinuet if brd.has_song_active() => {
            brd.repertoire.min(WANDERERS_REPERTOIRE_MAX)
        }
        _ => 0,
    };
    let song_flags = visible_song_flags(&brd);
    let radiant_finale_coda = coda_count(song_flags);

    let data = (song_remaining as u64)
        | ((repertoire as u64) << 32)
        | ((soul_voice as u64) << 40)
        | ((radiant_finale_coda as u64) << 48)
        | ((song_flags as u64) << 56);
    tracing::debug!(
        song_remaining,
        repertoire,
        soul_voice,
        radiant_finale_coda,
        song_flags,
        data,
        "Built Bard gauge"
    );
    data
}

/// Apply gauge actions from Lua scripts
pub(crate) fn apply_bard_gauge_action(
    combat_state: &mut PlayerCombatState,
    index: u8,
    amount: i32,
) {
    let brd = &mut combat_state.bard;
    match index {
        BARD_GAUGE_SOUL_VOICE => {
            let new_value = (brd.soul_voice as i32 + amount).clamp(0, MAX_SOUL_VOICE as i32);
            brd.soul_voice = new_value as u8;
        }
        _ => {}
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct BardCooldownUpdate {
    pub cooldown_group: u32,
    pub elapsed_centisec: u32,
    pub total_centisec: u32,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct BardActionUpdate {
    pub changed: bool,
    pub status_timer_refreshed: bool,
    pub cooldown_update: Option<BardCooldownUpdate>,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct BardRefreshResult {
    pub changed: bool,
    pub status_timer_refreshed: bool,
    pub cooldown_update: Option<BardCooldownUpdate>,
}

/// Update Bard runtime state on actor refresh.
pub(crate) fn refresh_bard_runtime_state_on_actor(
    owner_actor_id: ObjectId,
    actor: &mut NetworkedActor,
) -> BardRefreshResult {
    let level = actor.get_common_spawn().level;
    let NetworkedActor::Player {
        combat_state,
        status_effects,
        ..
    } = actor
    else {
        return BardRefreshResult::default();
    };

    let before = combat_state.bard.clone();
    refresh_bard_runtime_state(&mut combat_state.bard);
    let mut result = BardRefreshResult {
        changed: combat_state.bard != before,
        ..Default::default()
    };

    if before.hawk_eye_stacks > 0 && combat_state.bard.hawk_eye_stacks == 0 {
        result.status_timer_refreshed |= remove_status_if_present(status_effects, STATUS_HAWK_EYE);
        result.changed |= result.status_timer_refreshed;
    }

    if !combat_state.bard.has_song_active() {
        result.status_timer_refreshed |= remove_repertoire_status(status_effects);
        result.changed |= result.status_timer_refreshed;
        return result;
    }

    let tick_due = take_due_repertoire_tick(&mut combat_state.bard);
    if tick_due && combat_state.in_combat && fastrand::u8(0..100) < REPERTOIRE_PROC_CHANCE_PERCENT {
        let proc_update =
            apply_repertoire_proc(combat_state, status_effects, owner_actor_id, level);
        result.changed |= proc_update.changed;
        result.status_timer_refreshed |= proc_update.status_timer_refreshed;
        result.cooldown_update = proc_update.cooldown_update;
    }

    result
}

/// Check if the action is a Bard weaponskill (for Barrage consumption)
pub(crate) fn is_bard_weaponskill(action_id: u32) -> bool {
    matches!(
        action_id,
        ACTION_HEAVY_SHOT
            | ACTION_BURST_SHOT
            | ACTION_REFULGENT_ARROW
            | ACTION_SHADOWBITE
            | ACTION_CAUSTIC_BITE
            | ACTION_STORMBITE
            | ACTION_IRON_JAWS
            | ACTION_APEX_ARROW
            | ACTION_BLAST_ARROW
    )
}

/// Get the status effect ID for the current song
pub(crate) fn get_song_status_id(song: BardSong) -> Option<u16> {
    match song {
        BardSong::None => None,
        BardSong::MagesBallad => Some(STATUS_MAGES_BALLAD),
        BardSong::ArmysPaeon => Some(STATUS_ARMYS_PAEON),
        BardSong::WanderersMinuet => Some(STATUS_WANDERERS_MINUET),
    }
}
