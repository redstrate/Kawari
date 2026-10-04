use diesel::{
    backend::Backend,
    deserialize::{self, FromSqlRow},
    expression::AsExpression,
    serialize,
    sql_types::Text,
    sqlite::Sqlite,
};
use mlua::{FromLua, Lua, LuaSerdeExt, UserData, UserDataFields};
use physis::savedata::chardat::CustomizeData;
use serde_json::{Value, json};

use crate::{customize_data_from_json, customize_data_to_json};

#[derive(Debug, Clone, AsExpression, FromSqlRow, Default)]
#[diesel(sql_type = Text)]
pub struct CharaMake {
    pub customize: CustomizeData,
    pub voice_id: i32,
    pub guardian: i32,
    pub birth_month: i32,
    pub birth_day: i32,
    pub classjob_id: i32,
    pub unk2: i32,
}

impl CharaMake {
    pub fn from_json(json: &str) -> Self {
        let v: Value = serde_json::from_str(json).unwrap();
        let content = &v["content"];

        Self {
            customize: customize_data_from_json(&content[0]),
            voice_id: content[1].as_str().unwrap().parse::<i32>().unwrap(),
            guardian: content[2].as_str().unwrap().parse::<i32>().unwrap(),
            birth_month: content[3].as_str().unwrap().parse::<i32>().unwrap(),
            birth_day: content[4].as_str().unwrap().parse::<i32>().unwrap(),
            classjob_id: content[5].as_str().unwrap().parse::<i32>().unwrap(),
            unk2: content[6].as_str().unwrap().parse::<i32>().unwrap(),
        }
    }

    pub fn to_json(&self) -> String {
        let content = json!([
            customize_data_to_json(&self.customize),
            self.voice_id.to_string(),
            self.guardian.to_string(),
            self.birth_month.to_string(),
            self.birth_day.to_string(),
            self.classjob_id.to_string(),
            self.unk2.to_string(),
        ]);

        let obj = json!({
            "classid": 118,
            "classname": "CharaMake",
            "content": content,
        });

        serde_json::to_string(&obj).unwrap()
    }
}

impl serialize::ToSql<Text, Sqlite> for CharaMake {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.to_json());
        Ok(serialize::IsNull::No)
    }
}

impl deserialize::FromSql<Text, Sqlite> for CharaMake {
    fn from_sql(mut bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        Ok(CharaMake::from_json(bytes.read_text()))
    }
}

impl FromLua for CharaMake {
    fn from_lua(value: mlua::Value, _: &Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => unreachable!(),
        }
    }
}

impl UserData for CharaMake {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("customize", |lua, this| lua.to_value(&this.customize));
        fields.add_field_method_set("customize", |lua, this, value| {
            this.customize = lua.from_value(value).unwrap();
            Ok(())
        });
    }
}

#[cfg(test)]
mod tests {
    use physis::race::{Gender, Race};

    use crate::lua::KawariLua;

    use super::*;

    #[test]
    fn read_charamake() {
        let json = "{\"classid\":118,\"classname\":\"CharaMake\",\"content\":[[\"1\",\"0\",\"1\",\"50\",\"1\",\"5\",\"161\",\"0\",\"3\",\"30\",\"103\",\"0\",\"0\",\"0\",\"1\",\"30\",\"4\",\"5\",\"2\",\"128\",\"35\",\"50\",\"0\",\"0\",\"0\",\"0\"],\"1\",\"1\",\"1\",\"1\",\"1\",\"1\"]}";

        let chara_make = CharaMake::from_json(json);
        assert_eq!(chara_make.customize.gender, Gender::Male);
        assert_eq!(chara_make.voice_id, 1);
        assert_eq!(chara_make.guardian, 1);
        assert_eq!(chara_make.birth_month, 1);
        assert_eq!(chara_make.birth_day, 1);
        assert_eq!(chara_make.classjob_id, 1);
        assert_eq!(chara_make.unk2, 1);
    }

    #[test]
    fn roundtrip_charamake() {
        let json = "{\"classid\":118,\"classname\":\"CharaMake\",\"content\":[[\"1\",\"0\",\"1\",\"50\",\"1\",\"5\",\"161\",\"0\",\"3\",\"30\",\"103\",\"0\",\"0\",\"0\",\"1\",\"30\",\"4\",\"5\",\"2\",\"128\",\"35\",\"50\",\"0\",\"0\",\"0\",\"0\"],\"1\",\"1\",\"1\",\"1\",\"1\",\"1\"]}";
        assert_eq!(CharaMake::from_json(json).to_json(), json);
    }

    #[test]
    fn lua_api_charamake() {
        let chara_make = CharaMake::default();

        let lua = KawariLua::new();
        lua.0.globals().set("chara_make", chara_make).unwrap();

        // The fields are actually available
        lua.0
            .load(
                r#"
            assert(chara_make.customize.race == "Hyur")
        "#,
            )
            .exec()
            .unwrap();

        // The fields can be written to in Lua...
        lua.0
            .load(
                r#"
            local new_customize = chara_make.customize
            new_customize.race = "Viera"
            chara_make.customize = new_customize
            assert(chara_make.customize.race == "Viera")
        "#,
            )
            .exec()
            .unwrap();

        // ... but available in Rust
        let chara_make: CharaMake = lua.0.globals().get("chara_make").unwrap();
        assert_eq!(chara_make.customize.race, Race::Viera);

        // And ditto if they're using integer representation (needed by certian GM commands and events)
        lua.0
            .load(
                r#"
            local new_customize = chara_make.customize
            new_customize.race = race_from_repr(1)
            chara_make.customize = new_customize
            assert(chara_make.customize.race == "Hyur")
        "#,
            )
            .exec()
            .unwrap();

        let chara_make: CharaMake = lua.0.globals().get("chara_make").unwrap();
        assert_eq!(chara_make.customize.race, Race::Hyur);
    }
}
