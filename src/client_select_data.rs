use binrw::binrw;
use serde_json::{Value, json};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ClientCustomizeData {
    pub race: i32,
    pub gender: i32,
    pub height: i32,
    pub subrace: i32,
    pub face: i32,
    pub hair: i32,
    pub enable_highlights: i32,
    pub skin_tone: i32,
    pub right_eye_color: i32,
    pub hair_tone: i32,
    pub highlights: i32,
    pub facial_features: i32,
    pub facial_feature_color: i32,
    pub eyebrows: i32,
    pub left_eye_color: i32,
    pub eyes: i32,
    pub nose: i32,
    pub jaw: i32,
    pub mouth: i32,
    pub lips_tone_fur_pattern: i32,
    pub race_feature_size: i32,
    pub race_feature_type: i32,
    pub bust: i32,
    pub face_paint: i32,
    pub face_paint_color: i32,
}

impl ClientCustomizeData {
    pub fn to_json(&self) -> Value {
        json!([
            self.race.to_string(),
            self.gender.to_string(),
            self.height.to_string(),
            self.subrace.to_string(),
            self.face.to_string(),
            self.enable_highlights.to_string(),
            self.skin_tone.to_string(),
            self.right_eye_color.to_string(),
            self.hair_tone.to_string(),
            self.highlights.to_string(),
            self.facial_features.to_string(),
            self.facial_feature_color.to_string(),
            self.eyebrows.to_string(),
            self.left_eye_color.to_string(),
            self.eyes.to_string(),
            self.nose.to_string(),
            self.jaw.to_string(),
            self.mouth.to_string(),
            self.lips_tone_fur_pattern.to_string(),
            self.race_feature_size.to_string(),
            self.race_feature_type.to_string(),
            self.bust.to_string(),
            self.face_paint.to_string(),
            self.face_paint_color.to_string(),
        ])
    }
}
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
    pub customize: ClientCustomizeData,
    pub unk12: i32,
    pub unk13: i32,
    pub unk14: [i32; 10],
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

        serde_json::to_string_pretty(&obj).unwrap()
    }
}
