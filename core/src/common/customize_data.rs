use physis::{
    race::{Gender, Race, Tribe},
    savedata::chardat::CustomizeData,
};
use serde_json::{Value, json};

pub fn customize_data_to_json(customize: &CustomizeData) -> Value {
    json!([
        (customize.race as u8).to_string(),
        (customize.gender as u8).to_string(),
        customize.age.to_string(),
        customize.height.to_string(),
        (customize.tribe as u8).to_string(),
        customize.face.to_string(),
        customize.hair.to_string(),
        (customize.enable_highlights as u8).to_string(),
        customize.skin_tone.to_string(),
        customize.right_eye_color.to_string(),
        customize.hair_tone.to_string(),
        customize.highlights.to_string(),
        customize.facial_features.to_string(),
        customize.facial_feature_color.to_string(),
        customize.eyebrows.to_string(),
        customize.left_eye_color.to_string(),
        customize.eyes.to_string(),
        customize.nose.to_string(),
        customize.jaw.to_string(),
        customize.mouth.to_string(),
        customize.lips_tone_fur_pattern.to_string(),
        customize.race_feature_size.to_string(),
        customize.race_feature_type.to_string(),
        customize.bust.to_string(),
        customize.face_paint.to_string(),
        customize.face_paint_color.to_string(),
    ])
}

pub fn customize_data_from_json(json: &Value) -> CustomizeData {
    CustomizeData {
        race: Race::from_repr(json[0].as_str().unwrap().parse::<u8>().unwrap()).unwrap(),
        gender: Gender::from_repr(json[1].as_str().unwrap().parse::<u8>().unwrap()).unwrap(),
        age: json[2].as_str().unwrap().parse::<u8>().unwrap(),
        height: json[3].as_str().unwrap().parse::<u8>().unwrap(),
        tribe: Tribe::from_repr(json[4].as_str().unwrap().parse::<u8>().unwrap()).unwrap(),
        face: json[5].as_str().unwrap().parse::<u8>().unwrap(),
        hair: json[6].as_str().unwrap().parse::<u8>().unwrap(),
        enable_highlights: json[7].as_str().unwrap().parse::<u8>().unwrap() == 1,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_customize_data() {
        let json = "[\"1\",\"1\",\"1\",\"46\",\"2\",\"2\",\"122\",\"0\",\"0\",\"137\",\"98\",\"0\",\"1\",\"0\",\"2\",\"137\",\"5\",\"5\",\"0\",\"131\",\"169\",\"0\",\"0\",\"100\",\"1\",\"5\"]";

        let customize = customize_data_from_json(&serde_json::from_str(json).unwrap());
        assert_eq!(customize.race, Race::Hyur);
        assert_eq!(customize.gender, Gender::Female);
        assert_eq!(customize.age, 1);
        assert_eq!(customize.height, 46);
        assert_eq!(customize.tribe, Tribe::Highlander);
        assert_eq!(customize.face, 2);
        assert_eq!(customize.hair, 122);
        assert_eq!(customize.enable_highlights, false);
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
            customize_data_to_json(&customize_data_from_json(
                &serde_json::from_str(json).unwrap()
            ))
            .to_string(),
            json
        );
    }
}
