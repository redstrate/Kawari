use binrw::binrw;

use crate::common::ObjectId;

#[binrw]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StatusEffect {
    /// Index into the Status Excel sheet.
    pub effect_id: u16,
    /// Arbitrary value.
    pub param: u16,
    /// How much time is remaining for this status effect.
    pub duration: f32,
    /// If valid, who gave this status effect.
    pub source_actor_id: ObjectId,
}

#[cfg(feature = "server")]
impl mlua::UserData for StatusEffect {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("param", |_, this| Ok(this.param));
    }
}
