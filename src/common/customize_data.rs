use binrw::binrw;
use serde_json::{Value, json};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct CustomizeData {
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

impl From<physis::savedata::chardat::CustomizeData> for CustomizeData {
    fn from(value: physis::savedata::chardat::CustomizeData) -> Self {
        Self {
            race: value.race as u8,
            gender: value.gender as u8,
            age: value.age,
            height: value.height,
            subrace: value.tribe as u8,
            face: value.face,
            hair: value.hair,
            enable_highlights: value.enable_highlights as u8,
            skin_tone: value.skin_tone,
            right_eye_color: value.right_eye_color,
            hair_tone: value.hair_tone,
            highlights: value.highlights,
            facial_features: value.facial_features,
            facial_feature_color: value.facial_feature_color,
            eyebrows: value.eyebrows,
            left_eye_color: value.left_eye_color,
            eyes: value.eyes,
            nose: value.nose,
            jaw: value.jaw,
            mouth: value.mouth,
            lips_tone_fur_pattern: value.lips_tone_fur_pattern,
            race_feature_size: value.race_feature_size,
            race_feature_type: value.race_feature_type,
            bust: value.bust,
            face_paint: value.face_paint,
            face_paint_color: value.face_paint_color,
        }
    }
}

impl CustomizeData {
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
            age: json[2].as_str().unwrap().parse::<u8>().unwrap(),
            height: json[3].as_str().unwrap().parse::<u8>().unwrap(),
            subrace: json[4].as_str().unwrap().parse::<u8>().unwrap(),
            face: json[5].as_str().unwrap().parse::<u8>().unwrap(),
            hair: json[6].as_str().unwrap().parse::<u8>().unwrap(),
            enable_highlights: json[7].as_str().unwrap().parse::<u8>().unwrap(),
            skin_tone: json[8].as_str().unwrap().parse::<u8>().unwrap(),
            right_eye_color: json[9].as_str().unwrap().parse::<u8>().unwrap(),
            hair_tone: json[10].as_str().unwrap().parse::<u8>().unwrap(),
            highlights: json[11].as_str().unwrap().parse::<u8>().unwrap(),
            facial_features: json[12].as_str().unwrap().parse::<u8>().unwrap(),
            facial_feature_color: json[13].as_str().unwrap().parse::<u8>().unwrap(),
            eyebrows: json[14].as_str().unwrap().parse::<u8>().unwrap(),
            left_eye_color: json[15].as_str().unwrap().parse::<u8>().unwrap(),
            eyes: json[16].as_str().unwrap().parse::<u8>().unwrap(),
            nose: json[17].as_str().unwrap().parse::<u8>().unwrap(),
            jaw: json[18].as_str().unwrap().parse::<u8>().unwrap(),
            mouth: json[19].as_str().unwrap().parse::<u8>().unwrap(),
            lips_tone_fur_pattern: json[20].as_str().unwrap().parse::<u8>().unwrap(),
            race_feature_size: json[21].as_str().unwrap().parse::<u8>().unwrap(),
            race_feature_type: json[22].as_str().unwrap().parse::<u8>().unwrap(),
            bust: json[23].as_str().unwrap().parse::<u8>().unwrap(),
            face_paint: json[24].as_str().unwrap().parse::<u8>().unwrap(),
            face_paint_color: json[25].as_str().unwrap().parse::<u8>().unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_customize_data() {
        let json = "[\"1\",\"1\",\"1\",\"46\",\"2\",\"2\",\"122\",\"0\",\"0\",\"137\",\"98\",\"0\",\"1\",\"0\",\"2\",\"137\",\"5\",\"5\",\"0\",\"131\",\"169\",\"0\",\"0\",\"100\",\"1\",\"5\"]";

        let customize = CustomizeData::from_json(&serde_json::from_str(json).unwrap());
        assert_eq!(customize.race, 1);
        assert_eq!(customize.gender, 1);
        assert_eq!(customize.age, 1);
        assert_eq!(customize.height, 46);
        assert_eq!(customize.subrace, 2);
        assert_eq!(customize.face, 2);
        assert_eq!(customize.hair, 122);
        assert_eq!(customize.enable_highlights, 0);
        assert_eq!(customize.skin_tone, 0);
        assert_eq!(customize.right_eye_color, 137);
        assert_eq!(customize.hair_tone, 98);
        assert_eq!(customize.highlights, 0);
        assert_eq!(customize.facial_features, 1);
        assert_eq!(customize.facial_feature_color, 0);
        assert_eq!(customize.eyebrows, 2);
        assert_eq!(customize.left_eye_color, 137);
        assert_eq!(customize.eyes, 5);
        assert_eq!(customize.nose, 5);
        assert_eq!(customize.jaw, 0);
        assert_eq!(customize.mouth, 131);
        assert_eq!(customize.lips_tone_fur_pattern, 169);
        assert_eq!(customize.race_feature_size, 0);
        assert_eq!(customize.race_feature_type, 0);
        assert_eq!(customize.bust, 100);
        assert_eq!(customize.face_paint, 1);
        assert_eq!(customize.face_paint_color, 5);
    }

    #[test]
    fn roundtrip_customize_data() {
        let json = "[\"1\",\"1\",\"1\",\"46\",\"2\",\"2\",\"122\",\"0\",\"0\",\"137\",\"98\",\"0\",\"1\",\"0\",\"2\",\"137\",\"5\",\"5\",\"0\",\"131\",\"169\",\"0\",\"0\",\"100\",\"1\",\"5\"]";
        assert_eq!(
            CustomizeData::from_json(&serde_json::from_str(json).unwrap())
                .to_json()
                .to_string(),
            json
        );
    }
}
