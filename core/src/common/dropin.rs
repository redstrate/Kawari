use crate::common::Position;
use serde::{Deserialize, Serialize};

/// A JSON file that appends an existing (and usually empty) LGB.
///
/// These can only add gathering points for now.
#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub struct DropIn {
    /// The path to the LGB this is appending.
    pub appends: String,
    /// The layers inside this dropin.
    pub layers: Vec<DropInLayer>,
}

/// Drop-in layer that appends one of `name`.
#[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
pub struct DropInLayer {
    name: String,
    pub objects: Vec<DropInObject>,
}

/// Drop-in object that can add new objects to a zone.
#[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
pub struct DropInObject {
    pub instance_id: u32,
    pub position: Position,
    pub rotation: f32,
    pub data: DropInObjectData,
}

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
#[serde(tag = "type")]
pub enum DropInObjectData {
    /// Represents a single gathering point.
    #[serde(rename = "gathering_point")]
    GatheringPoint {
        /// Index into the GatheringPoint Excel sheet.
        base_id: u32,
    },
    /// Represents a battle or event NPC.
    #[serde(rename = "npc")]
    Npc {
        /// Index into the BNpcBase sheet.
        base_id: u32,
        /// Index into the BNpcName sheet.
        name_id: u32,
        /// If set, overrides the normal HP scaling.
        hp: Option<u32>,
        /// Level of this NPC.
        level: u32,
        /// If true, this NPC does not spawn automatically.
        nonpop: bool,
        /// Whether this NPC is hostile or not.
        hostile: bool,
        /// Gimmick ID, see `gimmick_id` in `SpawnNpc`.
        gimmick_id: u32,
        /// How many other BNpcs can be linked in this family.
        max_links: u8,
        /// If not zero, specifies which family this BNpc is linked to.
        link_family: u8,
        /// How far the link family can be apart.
        link_range: u8,
        /// Whether to consider this a battle NPC.
        battle_npc: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_example() {
        let json = std::fs::read_to_string("../resources/data/tests/example_dropin.json").unwrap();
        let dropin: DropIn = serde_json::from_str(&json).unwrap();

        assert_eq!(
            dropin,
            DropIn {
                appends: "bg/ffxiv/sea_s1/fld/s1f1/level/planlive.lgb".to_string(),
                layers: vec![DropInLayer {
                    name: "CRF_MINING_LV20".to_string(),
                    objects: vec![DropInObject {
                        instance_id: 4001271,
                        position: Position {
                            x: -266.0561,
                            y: 29.50931,
                            z: -562.5141
                        },
                        rotation: 0.0,
                        data: DropInObjectData::GatheringPoint { base_id: 30001 }
                    }]
                }]
            }
        );
    }
}
