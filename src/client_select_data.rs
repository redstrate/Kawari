use binrw::binrw;
use serde_json::{Value, json};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ClientCustomizeData {
    pub race: u8,
    pub gender: u8,
    pub age: u8,
    pub height: u8,
    pub subrace: u8,
    pub face: u8,
    pub hair: u8,
    pub enable_highlights: u8,
    pub skin_tone: u8,
    pub right_eye_color: u8,
    pub hair_tone: u8,
    pub highlights: u8,
    pub facial_features: u8,
    pub facial_feature_color: u8,
    pub eyebrows: u8,
    pub left_eye_color: u8,
    pub eyes: u8,
    pub nose: u8,
    pub jaw: u8,
    pub mouth: u8,
    pub lips_tone_fur_pattern: u8,
    pub race_feature_size: u8,
    pub race_feature_type: u8,
    pub bust: u8,
    pub face_paint: u8,
    pub face_paint_color: u8,
}

impl ClientCustomizeData {
    pub fn to_json(&self) -> Value {
        json!([
            self.race.to_string(),
            self.gender.to_string(),
            self.age.to_string(),
            self.height.to_string(),
            self.subrace.to_string(),
            self.face.to_string(),
            self.hair.to_string(),
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

    pub fn from_json(json: &Value) -> Self {
        Self {
            race: json[0].as_str().unwrap().parse::<u8>().unwrap(),
            gender: json[1].as_str().unwrap().parse::<u8>().unwrap(),
            height: json[2].as_str().unwrap().parse::<u8>().unwrap(),
            subrace: json[3].as_str().unwrap().parse::<u8>().unwrap(),
            face: json[4].as_str().unwrap().parse::<u8>().unwrap(),
            hair: json[5].as_str().unwrap().parse::<u8>().unwrap(),
            enable_highlights: json[6].as_str().unwrap().parse::<u8>().unwrap(),
            skin_tone: json[7].as_str().unwrap().parse::<u8>().unwrap(),
            right_eye_color: json[8].as_str().unwrap().parse::<u8>().unwrap(),
            hair_tone: json[9].as_str().unwrap().parse::<u8>().unwrap(),
            highlights: json[10].as_str().unwrap().parse::<u8>().unwrap(),
            facial_features: json[11].as_str().unwrap().parse::<u8>().unwrap(),
            facial_feature_color: json[12].as_str().unwrap().parse::<u8>().unwrap(),
            eyebrows: json[13].as_str().unwrap().parse::<u8>().unwrap(),
            left_eye_color: json[14].as_str().unwrap().parse::<u8>().unwrap(),
            eyes: json[15].as_str().unwrap().parse::<u8>().unwrap(),
            nose: json[16].as_str().unwrap().parse::<u8>().unwrap(),
            jaw: json[17].as_str().unwrap().parse::<u8>().unwrap(),
            mouth: json[18].as_str().unwrap().parse::<u8>().unwrap(),
            lips_tone_fur_pattern: json[19].as_str().unwrap().parse::<u8>().unwrap(),
            race_feature_size: json[20].as_str().unwrap().parse::<u8>().unwrap(),
            race_feature_type: json[21].as_str().unwrap().parse::<u8>().unwrap(),
            bust: json[22].as_str().unwrap().parse::<u8>().unwrap(),
            face_paint: json[23].as_str().unwrap().parse::<u8>().unwrap(),
            face_paint_color: json[24].as_str().unwrap().parse::<u8>().unwrap(),
            age: 1,
        }
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
