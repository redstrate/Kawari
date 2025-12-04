use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::common::{CustomizeData, EquipDisplayFlag};

// TODO: this isn't really an enum in the game, nor is it a flag either. it's weird!
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
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

#[cfg(all(not(target_family = "wasm"), feature = "server"))]
impl rusqlite::types::FromSql for RemakeMode {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(Self::try_from(i32::column_result(value)?).unwrap())
    }
}

/// See <https://github.com/aers/FFXIVClientStructs/blob/main/FFXIVClientStructs/FFXIV/Application/Network/WorkDefinitions/ClientSelectData.cs>
#[derive(Debug)]
pub struct ClientSelectData {
    pub character_name: String,
    pub current_class: i32,
    pub class_levels: Vec<i32>,
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
    pub model_ids: Vec<u32>,
    pub equip_stain: Vec<u32>,
    pub glasses: Vec<u32>,
    pub remake_mode: RemakeMode, // TODO: upstream a comment about this to FFXIVClientStructs
    /// If above 0, then a message warns the user that they have X minutes left to remake their character.
    pub remake_minutes_remaining: i32,
    pub display_flags: EquipDisplayFlag,
    pub voice_id: i32, // presumably
    pub unk21: i32,
    pub world_name: String,
    pub unk22: i32,
    pub unk23: i32,
}

impl ClientSelectData {
    pub fn from_json(json: &str) -> Self {
        let v: Value = serde_json::from_str(json).unwrap();
        let content = &v["content"];

        Self {
            character_name: content[0].as_str().unwrap().to_string(),
            current_class: content[1].as_str().unwrap().parse::<i32>().unwrap(),
            class_levels: content[2]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap().parse::<i32>().unwrap_or_default())
                .collect(),
            race: content[3].as_str().unwrap().parse::<i32>().unwrap(),
            subrace: content[4].as_str().unwrap().parse::<i32>().unwrap(),
            gender: content[5].as_str().unwrap().parse::<i32>().unwrap(),
            birth_month: content[6].as_str().unwrap().parse::<i32>().unwrap(),
            birth_day: content[7].as_str().unwrap().parse::<i32>().unwrap(),
            guardian: content[8].as_str().unwrap().parse::<i32>().unwrap(),
            unk8: content[9].as_str().unwrap().parse::<i32>().unwrap(),
            unk9: content[10].as_str().unwrap().parse::<i32>().unwrap(),
            zone_id: content[11].as_str().unwrap().parse::<i32>().unwrap(),
            content_finder_condition: content[12].as_str().unwrap().parse::<i32>().unwrap(),
            customize: CustomizeData::from_json(&content[13]),
            model_main_weapon: content[14].as_str().unwrap().parse::<u64>().unwrap(),
            model_sub_weapon: content[15].as_str().unwrap().parse::<i32>().unwrap(),
            model_ids: content[16]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap().parse::<u32>().unwrap_or_default())
                .collect(),
            equip_stain: content[17]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap().parse::<u32>().unwrap_or_default())
                .collect(),
            glasses: content[18]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap().parse::<u32>().unwrap_or_default())
                .collect(),
            remake_mode: RemakeMode::try_from(
                content[19].as_str().unwrap().parse::<i32>().unwrap(),
            )
            .unwrap(),
            remake_minutes_remaining: content[20].as_str().unwrap().parse::<i32>().unwrap(),
            display_flags: EquipDisplayFlag::from_bits(
                content[21].as_str().unwrap().parse::<u16>().unwrap(),
            )
            .unwrap(),
            voice_id: content[22].as_str().unwrap().parse::<i32>().unwrap(),
            unk21: content[23].as_str().unwrap().parse::<i32>().unwrap(),
            world_name: content[24].as_str().unwrap().to_string(),
            unk22: content[25].as_str().unwrap().parse::<i32>().unwrap(),
            unk23: content[26].as_str().unwrap().parse::<i32>().unwrap(),
        }
    }

    pub fn to_json(&self) -> String {
        let content = json!([
            self.character_name,
            self.current_class.to_string(),
            self.class_levels
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>(),
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
            self.model_ids
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>(),
            self.equip_stain
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>(),
            self.glasses
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>(),
            (self.remake_mode as i32).to_string(),
            self.remake_minutes_remaining.to_string(),
            (self.display_flags.0).to_string(),
            self.voice_id.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_client_select_data() {
        let json = "{\"content\":[\"Lavenaa Warren\",\"1\",[\"0\",\"1\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\"],\"1\",\"2\",\"1\",\"1\",\"1\",\"8\",\"1\",\"0\",\"182\",\"0\",[\"1\",\"1\",\"1\",\"46\",\"2\",\"2\",\"122\",\"0\",\"0\",\"137\",\"98\",\"0\",\"1\",\"0\",\"2\",\"137\",\"5\",\"5\",\"0\",\"131\",\"169\",\"0\",\"0\",\"100\",\"1\",\"5\"],\"4297785545\",\"0\",[\"0\",\"131157\",\"131157\",\"131157\",\"131157\",\"131073\",\"131073\",\"131073\",\"65537\",\"131073\"],[\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\"],[\"0\",\"0\"],\"0\",\"0\",\"52\",\"93\",\"0\",\"\",\"0\",\"0\"],\"classname\":\"ClientSelectData\",\"classid\":116}";

        let chara_make = ClientSelectData::from_json(json);
        assert_eq!(chara_make.character_name, "Lavenaa Warren");
        assert_eq!(chara_make.current_class, 1);
        assert_eq!(
            chara_make.class_levels,
            vec![
                0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0
            ]
        );
        assert_eq!(chara_make.race, 1);
        assert_eq!(chara_make.subrace, 2);
        assert_eq!(chara_make.gender, 1);
        assert_eq!(chara_make.birth_month, 1);
        assert_eq!(chara_make.birth_day, 1);
        assert_eq!(chara_make.guardian, 8);
        assert_eq!(chara_make.unk8, 1);
        assert_eq!(chara_make.unk9, 0);
        assert_eq!(chara_make.zone_id, 182);
        assert_eq!(chara_make.content_finder_condition, 0);
        // TODO: test customize data
        assert_eq!(chara_make.model_main_weapon, 4297785545);
        assert_eq!(chara_make.model_sub_weapon, 0);
        assert_eq!(
            chara_make.model_ids,
            vec![
                0, 131157, 131157, 131157, 131157, 131073, 131073, 131073, 65537, 131073
            ]
        );
        assert_eq!(chara_make.equip_stain, vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(chara_make.glasses, vec![0, 0]);
        assert_eq!(chara_make.remake_mode, RemakeMode::None);
        assert_eq!(chara_make.remake_minutes_remaining, 0);
        assert_eq!(
            chara_make.display_flags,
            EquipDisplayFlag::HIDE_LEGACY_MARK | EquipDisplayFlag::UNK3 | EquipDisplayFlag::UNK4
        );
        assert_eq!(chara_make.voice_id, 93);
        assert_eq!(chara_make.unk21, 0);
        assert_eq!(chara_make.world_name, "");
        assert_eq!(chara_make.unk22, 0);
        assert_eq!(chara_make.unk23, 0);
    }

    #[test]
    fn roundtrip_client_select_data() {
        let json = "{\"content\":[\"Lavenaa Warren\",\"1\",[\"0\",\"1\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\"],\"1\",\"2\",\"1\",\"1\",\"1\",\"8\",\"1\",\"0\",\"182\",\"0\",[\"1\",\"1\",\"1\",\"46\",\"2\",\"2\",\"122\",\"0\",\"0\",\"137\",\"98\",\"0\",\"1\",\"0\",\"2\",\"137\",\"5\",\"5\",\"0\",\"131\",\"169\",\"0\",\"0\",\"100\",\"1\",\"5\"],\"4297785545\",\"0\",[\"0\",\"131157\",\"131157\",\"131157\",\"131157\",\"131073\",\"131073\",\"131073\",\"65537\",\"131073\"],[\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\",\"0\"],[\"0\",\"0\"],\"0\",\"0\",\"52\",\"93\",\"0\",\"\",\"0\",\"0\"],\"classname\":\"ClientSelectData\",\"classid\":116}";
        assert_eq!(ClientSelectData::from_json(json).to_json(), json);
    }
}
