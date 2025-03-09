use serde_json::{Value, json};

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

pub struct ClientSelectData {
    pub game_name_unk: String,
    pub version_maybe: i32,
    pub unk1: [i32; 30],
    pub unk2: i32,
    pub unk3: i32,
    pub unk4: i32,
    pub unk5: i32,
    pub unk6: i32,
    pub unk7: i32,
    pub unk8: i32,
    pub unk9: i32,
    pub unk10: i32,
    pub unk11: i32,
    pub customize: ClientCustomizeData,
    pub unk12: i32,
    pub unk13: i32,
    pub unk14: [i32; 10],
    pub unk15: i32,
    pub unk16: i32,
    pub unk17: i32,
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
            self.version_maybe.to_string(),
            self.unk1.map(|x| x.to_string()),
            self.unk2.to_string(),
            self.unk3.to_string(),
            self.unk4.to_string(),
            self.unk5.to_string(),
            self.unk6.to_string(),
            self.unk7.to_string(),
            self.unk8.to_string(),
            self.unk9.to_string(),
            self.unk10.to_string(),
            self.unk11.to_string(),
            self.customize.to_json(),
            self.unk12.to_string(),
            self.unk13.to_string(),
            self.unk14.map(|x| x.to_string()),
            self.unk15.to_string(),
            self.unk16.to_string(),
            self.unk17.to_string(),
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
