mod chat_handler;
pub use chat_handler::ChatHandler;

mod chat_connection;
pub use chat_connection::ChatConnection;

mod zone_connection;
use diesel::backend::Backend;
use diesel::deserialize::{self, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::serialize;
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;
use kawari::constants::CLASSJOB_ARRAY_SIZE;
use serde::{Deserialize, Serialize};
pub use zone_connection::{
    ObsfucationData, PlayerData, TeleportReason, ZoneConnection, spawn_allocator::SpawnAllocator,
};

mod database;
pub use database::{Content, Unlock, WorldDatabase};

pub mod lua;

mod event;
pub use event::Event;

mod status_effects;
pub use status_effects::StatusEffects;

mod server;
pub use server::server_main_loop;

mod custom_ipc_connection;
pub use custom_ipc_connection::CustomIpcConnection;

mod common;
pub use common::{ClientHandle, ClientId, FromServer, MessageInfo, ServerHandle, ToServer};

mod navmesh;
pub use navmesh::{Navmesh, NavmeshParams, NavmeshTile};

pub mod auracite;

/// Inventory and storage management.
pub mod inventory;

mod bitmask;
pub use bitmask::{Bitmask, QuestBitmask};

mod gamedata;
pub use gamedata::GameData;
pub use gamedata::{ItemInfo, ItemInfoQuery, TerritoryNameKind};

mod chara_make;
pub use chara_make::CharaMake;

mod client_select_data;
pub use client_select_data::{ClientSelectData, RemakeMode};

use crate::zone_connection::PersistentQuest;

/// Define a new SQL-compatible array with an optional initial size.
macro_rules! define_sql_array {
    // With initial size.
    ($array_name:ident, $element_type:ident, $initial_size:expr) => {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AsExpression, FromSqlRow)]
        #[diesel(sql_type = Text)]
        pub struct $array_name(pub Vec<$element_type>);

        impl Default for $array_name {
            fn default() -> Self {
                Self(vec![0; $initial_size])
            }
        }

        impl serialize::ToSql<Text, Sqlite> for $array_name {
            fn to_sql<'b>(
                &'b self,
                out: &mut serialize::Output<'b, '_, Sqlite>,
            ) -> serialize::Result {
                out.set_value(serde_json::to_string(&self).unwrap());
                Ok(serialize::IsNull::No)
            }
        }

        impl deserialize::FromSql<Text, Sqlite> for $array_name {
            fn from_sql(mut bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
                Ok(serde_json::from_str(bytes.read_text())
                    .ok()
                    .unwrap_or_default())
            }
        }
    };
    // Without initial size.
    ($array_name:ident, $element_type:ident) => {
        #[derive(
            Debug, Clone, PartialEq, Default, Serialize, Deserialize, AsExpression, FromSqlRow,
        )]
        #[diesel(sql_type = Text)]
        pub struct $array_name(pub Vec<$element_type>);

        impl serialize::ToSql<Text, Sqlite> for $array_name {
            fn to_sql<'b>(
                &'b self,
                out: &mut serialize::Output<'b, '_, Sqlite>,
            ) -> serialize::Result {
                out.set_value(serde_json::to_string(&self).unwrap());
                Ok(serialize::IsNull::No)
            }
        }

        impl deserialize::FromSql<Text, Sqlite> for $array_name {
            fn from_sql(mut bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
                Ok(serde_json::from_str(bytes.read_text())
                    .ok()
                    .unwrap_or_default())
            }
        }
    };
}

// Arrays we store in SQL.
define_sql_array!(ClassLevels, u16, CLASSJOB_ARRAY_SIZE);
define_sql_array!(ClassExperience, i32, CLASSJOB_ARRAY_SIZE);
define_sql_array!(ActiveQuests, PersistentQuest);
define_sql_array!(FavoriteAetherytes, u16);
