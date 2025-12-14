use crate::models::*;
use diesel::prelude::*;
use diesel::{Connection, SqliteConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use kawari::constants::MAX_EXPANSION;
use serde::Serialize;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub struct LoginDatabase {
    connection: SqliteConnection,
}

#[derive(Debug, PartialEq)]
pub enum LoginError {
    WrongUsername,
    WrongPassword,
    InternalError,
}

impl Default for LoginDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
pub struct SessionInformation {
    pub time: String,
    pub service: String,
}

impl LoginDatabase {
    /// Creates a new connection to the database, and creates tables as needed.
    pub fn new() -> Self {
        let mut connection =
            SqliteConnection::establish("login.db").expect("Failed to open database!");
        Self::create_tables(&mut connection);

        Self { connection }
    }

    /// Creates a new connection to a database, but in memory. Only meant for our own testing.
    #[cfg(test)]
    fn new_in_memory() -> Self {
        let mut connection =
            SqliteConnection::establish(":memory:").expect("Failed to open database!");
        Self::create_tables(&mut connection);

        Self { connection }
    }

    /// Setups up the initial database schema.
    fn create_tables(connection: &mut SqliteConnection) {
        connection.run_pending_migrations(MIGRATIONS).unwrap();
    }

    /// Generates a random account ID.
    fn generate_account_id() -> u32 {
        // NOTE: We pass account IDs as u64, however SQLite doesn't support u64. So we *want* to create u32 account IDs.
        fastrand::u32(..)
    }

    /// Adds a new user to the database with `username` and `password`.
    ///
    /// Returns false if the username was already taken.
    pub fn add_user(&mut self, username: &str, password: &str) -> bool {
        use crate::schema::{service_account, user};

        if self.check_username(username) {
            tracing::error!("Username {username} already taken!");
            return false;
        }

        let user_id = Self::generate_account_id();

        // add user
        {
            tracing::info!("Adding user with username {username}");

            if let Err(err) = diesel::insert_into(user::table)
                .values(&User {
                    id: user_id as i64,
                    username: username.to_string(),
                    password: password.to_string(),
                })
                .execute(&mut self.connection)
            {
                tracing::error!("While adding user: {err:?}");
                return false;
            }
        }

        // add service account
        {
            diesel::insert_into(service_account::table)
                .values(&ServiceAccount {
                    id: Self::generate_account_id() as i64,
                    user_id: user_id as i64,
                    max_ex: MAX_EXPANSION as i32,
                })
                .execute(&mut self.connection)
                .unwrap();
        }

        true
    }

    /// Login as a user, and returns a session id if successful.
    ///
    /// `service` is the purpose of this login.
    /// `username` and `password` is the user's credentials.
    pub fn login_user(
        &mut self,
        service: &str,
        for_username: &str,
        for_password: &str,
    ) -> Result<String, LoginError> {
        use crate::schema::user::dsl::*;

        tracing::info!("Finding user with username {for_username}");

        if let Ok(selected_user) = user
            .filter(username.eq(for_username))
            .select(User::as_select())
            .first(&mut self.connection)
        {
            if selected_user.password == for_password {
                return self
                    .create_session(service, selected_user.id as u64)
                    .ok_or(LoginError::InternalError);
            } else {
                return Err(LoginError::WrongPassword);
            }
        }

        Err(LoginError::WrongUsername)
    }

    /// Generates a random session ID.
    fn generate_sid() -> String {
        let random_id: String =
            String::from_utf8((0..56).map(|_| fastrand::alphanumeric() as u8).collect())
                .expect("Failed to create random SID");
        random_id.to_lowercase()
    }

    /// Create a new session for user, which replaces the last one (if any) of a given `service`
    pub fn create_session(&mut self, service: &str, user_id: u64) -> Option<String> {
        use crate::schema::session;

        let sid = Self::generate_sid();
        let time = diesel::select(datetime())
            .get_result::<String>(&mut self.connection)
            .unwrap();

        // Delete an existing session if needed
        let _ = diesel::delete(session::table)
            .filter(crate::schema::session::dsl::user_id.eq(user_id as i64))
            .filter(crate::schema::session::dsl::service.eq(service))
            .execute(&mut self.connection);

        diesel::insert_into(session::table)
            .values(&Session {
                user_id: user_id as i64,
                time,
                service: service.to_string(),
                sid: sid.clone(),
            })
            .execute(&mut self.connection)
            .unwrap();

        tracing::info!("Created new session for account {user_id}: {sid}");

        Some(sid)
    }

    /// Gets the service account list
    pub fn check_session(
        &mut self,
        for_service: &str,
        from_sid: &str,
    ) -> Vec<kawari::ipc::lobby::ServiceAccount> {
        // get user id
        let Ok(found_user_id) = ({
            use crate::schema::session::dsl::*;

            session
                .filter(service.eq(for_service))
                .filter(sid.eq(from_sid))
                .select(user_id)
                .first::<i64>(&mut self.connection)
        }) else {
            return Vec::default();
        };

        // service accounts
        {
            use crate::schema::service_account::dsl::*;

            if let Ok(service_accounts) = service_account
                .filter(user_id.eq(found_user_id))
                .select(ServiceAccount::as_select())
                .load(&mut self.connection)
            {
                service_accounts
                    .iter()
                    .enumerate()
                    .map(|(i, x)| kawari::ipc::lobby::ServiceAccount {
                        id: x.id as u64,
                        index: i as u32,
                        name: format!("FINAL FANTASY XIV {}", i + 1), // TODO: don't add the "1" if you only have one service account
                    })
                    .collect()
            } else {
                Vec::default()
            }
        }
    }

    /// Checks if a username is taken
    pub fn check_username(&mut self, for_username: &str) -> bool {
        use crate::schema::user::dsl::*;

        user.filter(username.eq(for_username))
            .count()
            .get_result::<i64>(&mut self.connection)
            .unwrap_or_default()
            > 0
    }

    /// Returns the user ID associated with `sid`, or None if it's invalid or not found.
    pub fn get_user_id(&mut self, for_sid: &str) -> Option<u64> {
        use crate::schema::session::dsl::*;

        session
            .filter(sid.eq(for_sid))
            .select(user_id)
            .first::<i64>(&mut self.connection)
            .map(|x| x as u64)
            .ok()
    }

    pub fn get_username(&mut self, for_user_id: u64) -> String {
        use crate::schema::user::dsl::*;

        user.filter(id.eq(for_user_id as i64))
            .select(username)
            .first::<String>(&mut self.connection)
            .unwrap_or_default()
    }

    // TODO: only returns one account right now
    pub fn get_service_account(&mut self, for_user_id: u64) -> u64 {
        use crate::schema::service_account::dsl::*;

        service_account
            .filter(user_id.eq(for_user_id as i64))
            .select(id)
            .first::<i64>(&mut self.connection)
            .unwrap_or_default() as u64
    }

    /// Gets the current session list, at some point it will return past sessions too.
    pub fn get_sessions(&mut self, for_user_id: u64) -> Vec<SessionInformation> {
        use crate::schema::session::dsl::*;

        if let Ok(sessions) = session
            .filter(user_id.eq(for_user_id as i64))
            .select(Session::as_select())
            .load(&mut self.connection)
        {
            sessions
                .iter()
                .map(|x| SessionInformation {
                    time: x.time.clone(),
                    service: x.service.clone(),
                })
                .collect()
        } else {
            Vec::default()
        }
    }

    /// Simply checks if this is a valid session or not.
    pub fn is_session_valid(&mut self, for_service: &str, for_sid: &str) -> bool {
        use crate::schema::session::dsl::*;

        session
            .filter(sid.eq(for_sid))
            .filter(service.eq(for_service))
            .count()
            .first::<i64>(&mut self.connection)
            .unwrap_or_default()
            > 0
    }

    /// Revokes a given `service` from the active session list for the `user_id`.
    pub fn revoke_session(&mut self, for_user_id: u64, for_service: &str) {
        use crate::schema::session::dsl::*;

        diesel::delete(
            session
                .filter(user_id.eq(for_user_id as i64))
                .filter(service.eq(for_service)),
        )
        .execute(&mut self.connection)
        .unwrap();

        tracing::info!("Revoked {for_service} for user {for_user_id}!");
    }

    /// Deletes the given `user_id` and also scrubs their service accounts.
    pub fn delete_user(&mut self, for_user_id: u64) {
        // Delete service accounts
        {
            use crate::schema::service_account::dsl::*;
            diesel::delete(service_account.filter(user_id.eq(for_user_id as i64)))
                .execute(&mut self.connection)
                .unwrap();
        }

        // Delete sessions
        {
            use crate::schema::session::dsl::*;
            diesel::delete(session.filter(user_id.eq(for_user_id as i64)))
                .execute(&mut self.connection)
                .unwrap();
        }

        // Delete user
        {
            use crate::schema::user::dsl::*;
            diesel::delete(user.filter(id.eq(for_user_id as i64)))
                .execute(&mut self.connection)
                .unwrap();
        }

        tracing::info!("Deleted user {for_user_id}!");
    }

    /// Grabs basic information about every user in the database.
    pub fn get_users(&mut self) -> Vec<kawari::common::User> {
        use crate::schema::user::dsl::*;

        if let Ok(users) = user.select(User::as_select()).load(&mut self.connection) {
            users
                .iter()
                .map(|x| kawari::common::User {
                    id: x.id as u32,
                    username: x.username.clone(),
                })
                .collect()
        } else {
            Vec::default()
        }
    }

    /// Returns the max expansion level for the `service_account_id`
    pub fn get_max_expansion(&mut self, for_service_account_id: u64) -> Option<u8> {
        use crate::schema::service_account::dsl::*;

        Some(
            service_account
                .filter(id.eq(for_service_account_id as i64))
                .select(ServiceAccount::as_select())
                .first(&mut self.connection)
                .ok()?
                .max_ex as u8,
        )
    }

    /// Returns the max expansion level for this `user_id`.
    ///
    /// This takes the highest expansion level from all service accounts.
    pub fn get_user_max_expansion(&mut self, for_user_id: u64) -> Option<u8> {
        use crate::schema::service_account::dsl::*;

        Some(
            service_account
                .filter(user_id.eq(for_user_id as i64))
                .select(ServiceAccount::as_select())
                .first(&mut self.connection)
                .ok()?
                .max_ex as u8,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVICE_NAME: &'static str = "Unit Test";

    #[test]
    fn test_login() {
        let mut database = LoginDatabase::new_in_memory();

        // No users exist in the database yet.
        assert_eq!(
            database.login_user(SERVICE_NAME, "test", "test"),
            Err(LoginError::WrongUsername)
        );

        // Now add said user, the login should now succeed.
        assert!(database.add_user("test", "test"));
        assert!(database.login_user(SERVICE_NAME, "test", "test").is_ok());

        // But the same user with the wrong password should fail!
        assert_eq!(
            database.login_user(SERVICE_NAME, "test", "wrong"),
            Err(LoginError::WrongPassword)
        );
    }

    #[test]
    fn test_username_check() {
        let mut database = LoginDatabase::new_in_memory();
        assert!(database.add_user("test", "test"));

        // Adding the same username should fail!
        assert!(!database.add_user("test", "test"));
    }

    #[test]
    fn test_sessions() {
        tracing_subscriber::fmt::init();

        let mut database = LoginDatabase::new_in_memory();
        assert!(database.add_user("test", "test"));

        // User should be able to login.
        let sid = database.login_user(SERVICE_NAME, "test", "test");
        assert!(sid.is_ok());
        let sid = sid.unwrap();

        // This SID we just got should be valid.
        assert!(database.is_session_valid(SERVICE_NAME, &sid));
        // The same SID but with a different service name should be invalid.
        assert!(!database.is_session_valid("Something Other", &sid));
        // The same service name, but with an invalid SID should obviously be invalid.
        assert!(!database.is_session_valid(SERVICE_NAME, "abc"));

        // If we login with another session:
        let other_sid = database.login_user("Unit Test 2", "test", "test");
        assert!(other_sid.is_ok());
        let other_sid = other_sid.unwrap();

        // Both sessions should be valid:
        assert!(database.is_session_valid(SERVICE_NAME, &sid));
        assert!(database.is_session_valid("Unit Test 2", &other_sid));
    }

    #[test]
    fn test_delete_user() {
        let mut database = LoginDatabase::new_in_memory();

        // User shouldn't exist in the database yet.
        assert_eq!(
            database.login_user(SERVICE_NAME, "test", "test"),
            Err(LoginError::WrongUsername)
        );

        // Now add said user, the login should now succeed.
        assert!(database.add_user("test", "test"));

        // User should be able to login.
        let sid = database.login_user(SERVICE_NAME, "test", "test");
        assert!(sid.is_ok());
        let user_id = database.get_user_id(&sid.unwrap());
        assert!(user_id.is_some());
        let user_id = user_id.unwrap();

        // Delete the user...
        database.delete_user(user_id);

        // And we should be denied login again.
        assert_eq!(
            database.login_user(SERVICE_NAME, "test", "test"),
            Err(LoginError::WrongUsername)
        );
    }
}
