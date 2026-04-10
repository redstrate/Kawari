mod character;
mod friends;
mod linkshell;
mod mail;

mod models;
pub use models::{
    AetherCurrent, Aetheryte, Character, ClassJob, Companion, Content, Friends, GrandCompany,
    Mentor, Quest, SearchInfo, Unlock, Volatile,
};

mod schema;
mod social;

use diesel::{Connection, QueryDsl, RunQueryDsl, SqliteConnection, prelude::*};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use kawari::common::ObjectId;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub struct WorldDatabase {
    connection: SqliteConnection,
}

impl Default for WorldDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldDatabase {
    pub fn new() -> Self {
        let mut connection =
            SqliteConnection::establish("world.db").expect("Failed to open database!");

        connection.run_pending_migrations(MIGRATIONS).unwrap();

        Self { connection }
    }

    fn generate_content_id() -> u32 {
        fastrand::u32(..)
    }

    fn generate_actor_id() -> ObjectId {
        ObjectId(fastrand::u32(..))
    }

    /// returns
    pub fn find_service_account(&mut self, for_content_id: u64) -> u64 {
        use schema::character::dsl::*;

        character
            .filter(content_id.eq(for_content_id as i64))
            .select(service_account_id)
            .first::<i64>(&mut self.connection)
            .unwrap_or_default() as u64
    }

    pub fn do_cleanup_tasks(&mut self) {
        // Ensure the most volatile aspects of the db are reset to a clean state.
        // We expect these to be "offline" as the initial state elsewhere for things like the online player count and friend lists to function correctly.
        {
            use schema::volatile::dsl::*;

            diesel::update(volatile)
                .set(is_online.eq(false))
                .execute(&mut self.connection)
                .unwrap();
        }

        // Clean up orphaned linkshells with no members that were missed somehow. This should theoretically not happen without manual database edits.
        {
            use schema::linkshell_members::dsl::*;

            for (orphaned_linkshell_id, _) in self.find_all_linkshells() {
                if let Ok(members) = linkshell_members
                    .select(models::LinkshellMembers::as_select())
                    .filter(linkshell_id.eq(orphaned_linkshell_id as i64))
                    .load(&mut self.connection)
                    && members.is_empty()
                {
                    tracing::info!(
                        "Found orphaned linkshell {orphaned_linkshell_id} with zero members, cleaning it up now."
                    );
                    self.remove_linkshell(orphaned_linkshell_id);
                }
            }

            // TODO: Auto-promote new owners in linkshells that don't have owners, which should theoretically not happen without manual database edits.
        }
    }
}

#[declare_sql_function]
extern "SQL" {
    fn datetime() -> diesel::sql_types::Text;
}

#[declare_sql_function]
extern "SQL" {
    fn unixepoch() -> diesel::sql_types::BigInt;
}
