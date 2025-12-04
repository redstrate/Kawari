use binrw::binrw;

use crate::common::ObjectId;

#[binrw]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StatusEffect {
    pub effect_id: u16,
    pub param: u16,
    pub duration: f32,
    pub source_actor_id: ObjectId,
}

#[cfg(feature = "server")]
impl mlua::UserData for StatusEffect {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("param", |_, this| Ok(this.param));
    }
}
