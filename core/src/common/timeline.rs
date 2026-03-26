use serde::{Deserialize, Serialize};

/// A JSON file that appends an existing (and usually empty) LGB.
///
/// These can only add gathering points for now.
#[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
pub struct Timeline {
    /// Which action is used for the auto-attack. Index into the Action Excel sheet.
    pub autoattack_action_id: u32,
    /// Whether the timeline always plays.
    pub timeline_always_plays: bool,
    /// The timeline points.
    pub timepoints: Vec<Timepoint>,
    /// A series of actions (to play in sequence) on death.
    #[serde(default)]
    pub on_death: Vec<TimepointData>,
}

impl Timeline {
    /// Duration of the entire timeline in seconds.
    pub fn duration(&self) -> i32 {
        // TODO: maybe don't calculate it this way?
        let mut duration = 0;
        for point in &self.timepoints {
            duration = duration.max(point.time);
        }

        duration
    }

    /// Returns all points happening at the specified time, if any.
    pub fn points_at(&self, point: i32) -> Vec<&Timepoint> {
        self.timepoints.iter().filter(|x| x.time == point).collect()
    }
}

/// Represents a point on the timeline.
#[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
pub struct Timepoint {
    pub time: i32,
    pub data: TimepointData,
}

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
#[serde(tag = "type")]
pub enum TimepointData {
    /// Represents doing an action.
    #[serde(rename = "action")]
    Action {
        /// Index into the Action Excel sheet.
        action_id: u32,
    },
    /// Animates timelines for the gimmick this actor is bound to, such as Giant Clams.
    #[serde(rename = "timeline_state")]
    TimelineState { states: Vec<u32> },
    /// Changes the invulnerability state of this NPC.
    #[serde(rename = "invulnerability")]
    Invulnerability { invulnerable: bool },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_example() {
        let json =
            std::fs::read_to_string("../resources/data/tests/example_timeline.json").unwrap();
        let timeline: Timeline = serde_json::from_str(&json).unwrap();

        assert_eq!(
            timeline,
            Timeline {
                autoattack_action_id: 872,
                timeline_always_plays: false,
                on_death: Vec::default(),
                timepoints: vec![Timepoint {
                    time: 20,
                    data: TimepointData::Action { action_id: 872 }
                }]
            }
        );
    }
}
