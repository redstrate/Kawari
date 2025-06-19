use crate::common::ObjectId;

#[derive(Clone, Copy, Debug, Default)]
pub struct Actor {
    pub id: ObjectId,
    pub hp: u32,
    pub spawn_index: u32, // TODO: local to each connection, terrible place to put this
}
