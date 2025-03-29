use crate::common::ObjectId;

#[derive(Clone, Copy, Debug)]
pub struct Actor {
    pub id: ObjectId,
    pub hp: u32,
}
