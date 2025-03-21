use serde_json::{Value, json};

use crate::common::CustomizeData;

#[derive(Debug)]
pub struct CharaMake {
    pub customize: CustomizeData,
    pub unk1: i32, // always 1?
    pub guardian: i32,
    pub birth_month: i32,
    pub classjob: i32,
    pub birth_day: i32,
    pub unk6: i32, // always 1?
}

impl CharaMake {
    pub fn from_json(json: &str) -> Self {
        let v: Value = serde_json::from_str(json).unwrap();
        let content = &v["content"];

        Self {
            customize: CustomizeData::from_json(&content[0]),
            unk1: content[1].as_str().unwrap().parse::<i32>().unwrap(),
            guardian: content[2].as_str().unwrap().parse::<i32>().unwrap(),
            birth_month: content[3].as_str().unwrap().parse::<i32>().unwrap(),
            classjob: content[4].as_str().unwrap().parse::<i32>().unwrap(),
            birth_day: content[5].as_str().unwrap().parse::<i32>().unwrap(),
            unk6: content[6].as_str().unwrap().parse::<i32>().unwrap(),
        }
    }

    pub fn to_json(&self) -> String {
        let content = json!([
            self.customize.to_json(),
            self.unk1,
            self.guardian,
            self.birth_month,
            self.classjob,
            self.birth_day,
            self.unk6,
        ]);

        let obj = json!({
            "content": content,
            "classname": "CharaMake",
            "classid": 118,
        });

        serde_json::to_string(&obj).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_charamake() {
        let json = "{\"classid\":118,\"classname\":\"CharaMake\",\"content\":[[\"1\",\"0\",\"1\",\"50\",\"1\",\"5\",\"161\",\"0\",\"3\",\"30\",\"103\",\"0\",\"0\",\"0\",\"1\",\"30\",\"4\",\"5\",\"2\",\"128\",\"35\",\"50\",\"0\",\"0\",\"0\",\"0\"],\"1\",\"1\",\"1\",\"1\",\"1\",\"1\"]}";

        let chara_make = CharaMake::from_json(json);
        assert_eq!(chara_make.customize.gender, 0);
        assert_eq!(chara_make.classjob, 1);

        // TODO: add more asserts
    }
}
