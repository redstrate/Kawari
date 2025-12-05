use kawari::{common::User, constants::MAX_EXPANSION, ipc::lobby::ServiceAccount};
use parking_lot::Mutex;
use rusqlite::Connection;
use serde::Serialize;

pub struct LoginDatabase {
    connection: Mutex<Connection>,
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
        let connection = Connection::open("login.db").expect("Failed to open database!");

        Self::create_tables(&connection);

        Self {
            connection: Mutex::new(connection),
        }
    }

    /// Creates a new connection to a database, but in memory. Only meant for our own testing.
    #[cfg(test)]
    fn new_in_memory() -> Self {
        let connection = Connection::open_in_memory().expect("Failed to open database!");

        Self::create_tables(&connection);

        Self {
            connection: Mutex::new(connection),
        }
    }

    /// Setups up the initial database schema.
    fn create_tables(connection: &Connection) {
        // Create users table
        {
            let query = "CREATE TABLE IF NOT EXISTS user (id INTEGER PRIMARY KEY, username TEXT, password TEXT);";
            connection.execute(query, ()).unwrap();
        }

        // Create active sessions table
        {
            let query = "CREATE TABLE IF NOT EXISTS session (user_id INTEGER, time TEXT, service TEXT, sid TEXT, PRIMARY KEY(user_id, service));";
            connection.execute(query, ()).unwrap();
        }

        // Create service accounts table
        {
            let query = "CREATE TABLE IF NOT EXISTS service_account (id INTEGER PRIMARY KEY, user_id INTEGER, max_ex INTEGER);";
            connection.execute(query, ()).unwrap();
        }
    }

    /// Generates a random account ID.
    fn generate_account_id() -> u32 {
        // NOTE: We pass account IDs as u64, however SQLite doesn't support u64. So we *want* to create u32 account IDs.
        fastrand::u32(..)
    }

    /// Adds a new user to the database with `username` and `password`.
    ///
    /// Returns false if the username was already taken.
    pub fn add_user(&self, username: &str, password: &str) -> bool {
        if self.check_username(username) {
            return false;
        }

        let user_id = Self::generate_account_id();

        // add user
        {
            let connection = self.connection.lock();

            tracing::info!("Adding user with username {username}");

            let query = "INSERT INTO user VALUES (?1, ?2, ?3);";
            connection
                .execute(query, (user_id, username, password))
                .expect("Failed to write user to database!");
        }

        // add service account
        {
            let connection = self.connection.lock();

            let query = "INSERT INTO service_account VALUES (?1, ?2, ?3);";
            connection
                .execute(query, (Self::generate_account_id(), user_id, MAX_EXPANSION))
                .expect("Failed to write service account to database!");
        }

        true
    }

    /// Login as a user, and returns a session id if successful.
    ///
    /// `service` is the purpose of this login.
    /// `username` and `password` is the user's credentials.
    pub fn login_user(
        &self,
        service: &str,
        username: &str,
        password: &str,
    ) -> Result<String, LoginError> {
        let selected_row: Result<(u64, String), rusqlite::Error>;

        tracing::info!("Finding user with username {username}");

        {
            let connection = self.connection.lock();

            let mut stmt = connection
                .prepare("SELECT id, password FROM user WHERE username = ?1")
                .map_err(|_err| LoginError::WrongUsername)?;
            selected_row = stmt.query_row((username,), |row| Ok((row.get(0)?, row.get(1)?)));
        }

        if let Ok((id, their_password)) = selected_row {
            if their_password == password {
                return self
                    .create_session(service, id)
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
    pub fn create_session(&self, service: &str, user_id: u64) -> Option<String> {
        let connection = self.connection.lock();

        let sid = Self::generate_sid();

        connection
            .execute(
                "INSERT OR REPLACE INTO session VALUES (?1, datetime('now'), ?2, ?3);",
                (user_id, service, &sid),
            )
            .ok()?;

        tracing::info!("Created new session for account {user_id}: {sid}");

        Some(sid)
    }

    /// Gets the service account list
    pub fn check_session(&self, service: &str, sid: &str) -> Vec<ServiceAccount> {
        let connection = self.connection.lock();

        // get user id
        let user_id: u64;
        {
            let mut stmt = connection
                .prepare("SELECT user_id FROM session WHERE service = ?1 AND sid = ?2")
                .unwrap();
            if let Ok(found_user_id) = stmt.query_row((service, sid), |row| row.get(0)) {
                user_id = found_user_id;
            } else {
                return Vec::default();
            }
        }

        // service accounts
        {
            let mut stmt = connection
                .prepare("SELECT id FROM service_account WHERE user_id = ?1")
                .ok()
                .unwrap();
            let accounts = stmt.query_map((user_id,), |row| row.get(0)).unwrap();

            let mut service_accounts = Vec::new();
            for (index, id) in accounts.enumerate() {
                service_accounts.push(ServiceAccount {
                    id: id.unwrap(),
                    index: index as u32,
                    name: format!("FINAL FANTASY XIV {}", index + 1), // TODO: don't add the "1" if you only have one service account
                });
            }

            service_accounts
        }
    }

    /// Checks if a username is taken
    pub fn check_username(&self, username: &str) -> bool {
        let connection = self.connection.lock();

        let mut stmt = connection
            .prepare("SELECT id FROM user WHERE username = ?1")
            .ok()
            .unwrap();
        let selected_row: Result<u64, rusqlite::Error> =
            stmt.query_row((username,), |row| row.get(0));

        selected_row.is_ok()
    }

    /// Returns the user ID associated with `sid`, or None if it's invalid or not found.
    pub fn get_user_id(&self, sid: &str) -> Option<u64> {
        let connection = self.connection.lock();

        let mut stmt = connection
            .prepare("SELECT user_id FROM session WHERE sid = ?1")
            .ok()
            .unwrap();
        stmt.query_row((sid,), |row| row.get(0)).ok()?
    }

    pub fn get_username(&self, user_id: u64) -> String {
        let connection = self.connection.lock();

        let mut stmt = connection
            .prepare("SELECT username FROM user WHERE id = ?1")
            .ok()
            .unwrap();
        stmt.query_row((user_id,), |row| row.get(0)).unwrap()
    }

    // TODO: only returns one account right now
    pub fn get_service_account(&self, user_id: u64) -> u64 {
        let connection = self.connection.lock();

        let mut stmt = connection
            .prepare("SELECT id FROM service_account WHERE user_id = ?1")
            .ok()
            .unwrap();
        stmt.query_row((user_id,), |row| row.get(0)).unwrap()
    }

    /// Gets the current session list, at some point it will return past sessions too.
    pub fn get_sessions(&self, user_id: u64) -> Vec<SessionInformation> {
        let connection = self.connection.lock();

        let mut stmt = connection
            .prepare("SELECT time, service FROM session WHERE user_id = ?1 ORDER BY time DESC;")
            .ok()
            .unwrap();
        if let Ok(mut rows) = stmt.query((user_id,)) {
            let mut info = Vec::new();
            while let Some(row) = rows.next().unwrap() {
                info.push(SessionInformation {
                    time: row.get(0).unwrap(),
                    service: row.get(1).unwrap(),
                });
            }
            info
        } else {
            Vec::default()
        }
    }

    /// Simply checks if this is a valid session or not.
    pub fn is_session_valid(&self, service: &str, sid: &str) -> bool {
        let connection = self.connection.lock();

        let mut stmt = connection
            .prepare("SELECT user_id FROM session WHERE service = ?1 AND sid = ?2")
            .unwrap();
        stmt.query_row((service, sid), |row| row.get::<usize, u32>(0))
            .is_ok()
    }

    /// Revokes a given `service` from the active session list for the `user_id`.
    pub fn revoke_session(&self, user_id: u64, service: &str) {
        let connection = self.connection.lock();

        connection
            .execute(
                "DELETE FROM session WHERE user_id = ?1 AND service = ?2",
                (user_id, service),
            )
            .unwrap();

        tracing::info!("Revoked {service} for {user_id}!");
    }

    /// Deletes the given `user_id` and also scrubs their service accounts.
    pub fn delete_user(&self, user_id: u64) {
        let connection = self.connection.lock();

        // delete from users table
        connection
            .execute("DELETE FROM user WHERE id = ?1", (user_id,))
            .unwrap();

        // delete from service accounts table
        connection
            .execute("DELETE FROM service_account WHERE user_id = ?1", (user_id,))
            .unwrap();

        // delete from sessions table
        connection
            .execute("DELETE FROM session WHERE user_id = ?1", (user_id,))
            .unwrap();

        tracing::info!("Deleted {user_id}!");
    }

    /// Grabs basic information about every user in the database.
    pub fn get_users(&self) -> Vec<User> {
        let connection = self.connection.lock();

        let mut stmt = connection.prepare("SELECT id, username FROM user").unwrap();

        stmt.query_map((), |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
            })
        })
        .unwrap()
        .map(|x| x.unwrap())
        .collect()
    }

    /// Returns the max expansion level for the `service_account_id`
    pub fn get_max_expansion(&self, service_account_id: u64) -> Option<u8> {
        let connection = self.connection.lock();

        let mut stmt = connection
            .prepare("SELECT max_ex FROM service_account WHERE id = ?1")
            .unwrap();
        stmt.query_row((service_account_id,), |row| row.get(0))
            .ok()?
    }

    /// Returns the max expansion level for this `user_id`.
    ///
    /// This takes the highest expansion level from all service accounts.
    pub fn get_user_max_expansion(&self, user_id: u64) -> Option<u8> {
        let connection = self.connection.lock();

        let mut stmt = connection
            .prepare("SELECT MAX(max_ex) FROM service_account WHERE user_id = ?1")
            .unwrap();
        stmt.query_row((user_id,), |row| row.get(0)).ok()?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVICE_NAME: &'static str = "Unit Test";

    #[test]
    fn test_login() {
        let database = LoginDatabase::new_in_memory();

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
        let database = LoginDatabase::new_in_memory();
        assert!(database.add_user("test", "test"));

        // Adding the same username should fail!
        assert!(!database.add_user("test", "test"));
    }

    #[test]
    fn test_sessions() {
        let database = LoginDatabase::new_in_memory();
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
    }

    #[test]
    fn test_delete_user() {
        let database = LoginDatabase::new_in_memory();

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
