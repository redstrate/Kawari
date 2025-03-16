use serde_json::json;

use crate::common::CustomizeData;

/// See https://github.com/aers/FFXIVClientStructs/blob/main/FFXIVClientStructs/FFXIV/Application/Network/WorkDefinitions/ClientSelectData.cs
pub struct ClientSelectData {
    pub game_name_unk: String,
    pub current_class: i32,
    pub class_levels: [i32; 30],
    pub race: i32,
    pub subrace: i32,
    pub gender: i32,
    pub birth_month: i32,
    pub birth_day: i32,
    pub guardian: i32,
    pub unk8: i32,
    pub unk9: i32,
    pub zone_id: i32,
    pub unk11: i32,
    pub customize: CustomizeData,
    pub unk12: i32,
    pub unk13: i32,
    pub unk14: [i32; 10], // probably model ids
    pub unk15: i32,
    pub unk16: i32,
    /// If set to 1, the user is granted one opportunity to edit their character and are prompted to re-choose their class.
    pub legacy_character: i32,
    pub unk18: i32,
    pub unk19: i32,
    pub unk20: i32,
    pub unk21: String,
    pub unk22: i32,
    pub unk23: i32,
}

impl ClientSelectData {
    pub fn to_json(&self) -> String {
        let content = json!([
            self.game_name_unk,
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
            self.unk11.to_string(),
            self.customize.to_json(),
            self.unk12.to_string(),
            self.unk13.to_string(),
            self.unk14.map(|x| x.to_string()),
            self.unk15.to_string(),
            self.unk16.to_string(),
            self.legacy_character.to_string(),
            self.unk18.to_string(),
            self.unk19.to_string(),
            self.unk20.to_string(),
            self.unk21,
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
