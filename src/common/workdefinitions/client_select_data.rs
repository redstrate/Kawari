use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::common::CustomizeData;

// TODO: this isn't really an enum in the game, nor is it a flag either. it's weird!
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[repr(i32)]
pub enum RemakeMode {
    /// No remake options are available.
    None,
    /// "You are granted one opportunity to edit your character's race, tribe, gender, appearance, and name."
    EditAppearanceName = 1,
    /// "If you wish, you can edit your character's race, sex, and appearance."
    EditAppearance = 4,
}

impl TryFrom<i32> for RemakeMode {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::EditAppearanceName),
            4 => Ok(Self::EditAppearance),
            _ => Err(()),
        }
    }
}

/// See https://github.com/aers/FFXIVClientStructs/blob/main/FFXIVClientStructs/FFXIV/Application/Network/WorkDefinitions/ClientSelectData.cs
#[derive(Debug)]
pub struct ClientSelectData {
    pub character_name: String,
    pub current_class: i32,
    pub class_levels: [i32; 32],
    pub race: i32,
    pub subrace: i32,
    pub gender: i32,
    pub birth_month: i32,
    pub birth_day: i32,
    pub guardian: i32,
    pub unk8: i32,
    pub unk9: i32,
    pub zone_id: i32,
    /// Index into the ContentFinderCondition Excel sheet (presumably). But if != 0, then it does special things to the Lobby screen.
    /// The most notable is if your character can be remade, it says "Your character is currently bound by duty..."
    pub content_finder_condition: i32,
    pub customize: CustomizeData,
    pub model_main_weapon: u64,
    pub model_sub_weapon: i32,
    pub model_ids: [u32; 10],
    pub equip_stain: [u32; 10],
    pub glasses: [u32; 2],
    pub remake_mode: RemakeMode, // TODO: upstream a comment about this to FFXIVClientStructs
    /// If above 0, then a message warns the user that they have X minutes left to remake their character.
    pub remake_minutes_remaining: i32,
    pub voice_id: i32, // presumably
    pub unk20: i32,
    pub unk21: i32,
    pub world_name: String,
    pub unk22: i32,
    pub unk23: i32,
}

impl ClientSelectData {
    pub fn to_json(&self) -> String {
        let content = json!([
            self.character_name,
            self.current_class.to_string(),
            self.class_levels.map(|x| x.to_string()),
            self.race.to_string(),
            self.subrace.to_string(),
            self.gender.to_string(),
            self.birth_month.to_string(),
            self.birth_day.to_string(),
            self.guardian.to_string(),
            self.unk8.to_string(),
            self.unk9.to_string(),
            self.zone_id.to_string(),
            self.content_finder_condition.to_string(),
            self.customize.to_json(),
            self.model_main_weapon.to_string(),
            self.model_sub_weapon.to_string(),
            self.model_ids.map(|x| x.to_string()),
            self.equip_stain.map(|x| x.to_string()),
            self.glasses.map(|x| x.to_string()),
            (self.remake_mode as i32).to_string(),
            self.remake_minutes_remaining.to_string(),
            self.voice_id.to_string(),
            self.unk20.to_string(),
            self.unk21.to_string(),
            self.world_name,
            self.unk22.to_string(),
            self.unk23.to_string(),
        ]);

        let obj = json!({
            "content": content,
            "classname": "ClientSelectData",
            "classid": 116,
        });

        serde_json::to_string(&obj).unwrap()
    }
}
