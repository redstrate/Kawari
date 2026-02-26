use kawari::common::Position;
use serde::Deserialize;

/// A JSON file that appends an existing (and usually empty) LGB.
///
/// These can only add gathering points for now.
#[derive(Deserialize, PartialEq, Debug)]
pub struct DropIn {
    /// The path to the LGB this is appending.
    pub appends: String,
    /// The layers inside this dropin.
    pub layers: Vec<DropInLayer>,
}

/// Drop-in layer that appends one of `name`.
#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct DropInLayer {
    name: String,
    pub objects: Vec<DropInObject>,
}

/// Drop-in object that can add new objects to a zone.
#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct DropInObject {
    pub instance_id: u32,
    pub position: Position,
    pub rotation: f32,
    pub data: DropInObjectData,
}

#[derive(Deserialize, PartialEq, Debug, Clone)]
#[serde(tag = "type")]
pub enum DropInObjectData {
    /// Represents a single gathering point.
    #[serde(rename = "gathering_point")]
    GatheringPoint {
        /// Index into the GatheringPoint Excel sheet.
        base_id: u32,
    },
    /// Represents a battle NPC.
    #[serde(rename = "battle_npc")]
    BattleNpc {
        /// Index into the BNpcBase sheet.
        base_id: u32,
        /// Index into the BNpcName sheet.
        name_id: u32,
        /// HP of this NPC.
        hp: u32,
        /// Level of this NPC.
        level: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_example() {
        let json =
            std::fs::read_to_string("../../resources/data/tests/example_dropin.json").unwrap();
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
