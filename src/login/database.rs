use std::sync::Mutex;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::{constants::MAX_EXPANSION, ipc::lobby::ServiceAccount};

pub struct LoginDatabase {
    connection: Mutex<Connection>,
}

#[derive(Debug)]
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

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: u32,
    pub username: String,
}

impl LoginDatabase {
    pub fn new() -> Self {
        let connection = Connection::open("login.db").expect("Failed to open database!");

        // Create users table
        {
            let query = "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, username TEXT, password TEXT);";
            connection.execute(query, ()).unwrap();
        }

        // Create active sessions table
        {
            let query = "CREATE TABLE IF NOT EXISTS sessions (user_id INTEGER, time TEXT, service TEXT, sid TEXT, PRIMARY KEY(user_id, service));";
            connection.execute(query, ()).unwrap();
        }

        // Create service accounts table
        {
            let query = "CREATE TABLE IF NOT EXISTS service_accounts (id INTEGER PRIMARY KEY, user_id INTEGER, max_ex INTEGER);";
            connection.execute(query, ()).unwrap();
        }

        Self {
            connection: Mutex::new(connection),
        }
    }

    // NOE: We pass account IDs as u64, however SQLite doesn't support u64. So we *want* to create u32 account IDs.
    fn generate_account_id() -> u32 {
        fastrand::u32(..)
    }

    /// Adds a new user to the database.
    pub fn add_user(&self, username: &str, password: &str) {
        if self.check_username(username) {
            tracing::info!("{username} already taken!");
            return;
        }

        let user_id = Self::generate_account_id();

        // add user
        {
            let connection = self.connection.lock().unwrap();

            tracing::info!("Adding user with username {username}");

            let query = "INSERT INTO users VALUES (?1, ?2, ?3);";
            connection
                .execute(query, (user_id, username, password))
                .expect("Failed to write user to database!");
        }

        // add service account
        {
            let connection = self.connection.lock().unwrap();

            let query = "INSERT INTO service_accounts VALUES (?1, ?2, ?3);";
            connection
                .execute(query, (Self::generate_account_id(), user_id, MAX_EXPANSION))
                .expect("Failed to write service account to database!");
        }
    }

    /// Login as user, returns a session id.
    pub fn login_user(
        &self,
        service: &str,
        username: &str,
        password: &str,
    ) -> Result<String, LoginError> {
        let selected_row: Result<(u64, String), rusqlite::Error>;

        tracing::info!("Finding user with username {username}");

        {
            let connection = self.connection.lock().unwrap();

            let mut stmt = connection
                .prepare("SELECT id, password FROM users WHERE username = ?1")
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

    fn generate_sid() -> String {
        let random_id: String =
            String::from_utf8((0..56).map(|_| fastrand::alphanumeric() as u8).collect())
                .expect("Failed to create random SID");
        random_id.to_lowercase()
    }

    /// Create a new session for user, which replaces the last one (if any) of a given `service`
    pub fn create_session(&self, service: &str, user_id: u64) -> Option<String> {
        let connection = self.connection.lock().unwrap();

        let sid = Self::generate_sid();

        connection
            .execute(
                "INSERT OR REPLACE INTO sessions VALUES (?1, datetime('now'), ?2, ?3);",
                (user_id, service, &sid),
            )
            .ok()?;

        tracing::info!("Created new session for account {user_id}: {sid}");

        Some(sid)
    }

    /// Gets the service account list
    pub fn check_session(&self, service: &str, sid: &str) -> Vec<ServiceAccount> {
        let connection = self.connection.lock().unwrap();

        // get user id
        let user_id: u64;
        {
            let mut stmt = connection
                .prepare("SELECT user_id FROM sessions WHERE service = ?1 AND sid = ?2")
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
                .prepare("SELECT id FROM service_accounts WHERE user_id = ?1")
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
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT id FROM users WHERE username = ?1")
            .ok()
            .unwrap();
        let selected_row: Result<u64, rusqlite::Error> =
            stmt.query_row((username,), |row| row.get(0));

        selected_row.is_ok()
    }

    /// Returns the user ID associated with `sid`, or None if it's invalid or not found.
    pub fn get_user_id(&self, sid: &str) -> Option<u64> {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT user_id FROM sessions WHERE sid = ?1")
            .ok()
            .unwrap();
        stmt.query_row((sid,), |row| row.get(0)).ok()?
    }

    pub fn get_username(&self, user_id: u64) -> String {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT username FROM users WHERE id = ?1")
            .ok()
            .unwrap();
        stmt.query_row((user_id,), |row| row.get(0)).unwrap()
    }

    // TODO: only returns one account right now
    pub fn get_service_account(&self, user_id: u64) -> u64 {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT id FROM service_accounts WHERE user_id = ?1")
            .ok()
            .unwrap();
        stmt.query_row((user_id,), |row| row.get(0)).unwrap()
    }

    /// Gets the current session list, at some point it will return past sessions too.
    pub fn get_sessions(&self, user_id: u64) -> Vec<SessionInformation> {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT time, service FROM sessions WHERE user_id = ?1 ORDER BY time DESC;")
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
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT user_id FROM sessions WHERE service = ?1 AND sid = ?2")
            .unwrap();
        stmt.query_row((service, sid), |row| row.get::<usize, u32>(0))
            .is_ok()
    }

    /// Revokes a given `service` from the active session list for the `user_id`.
    pub fn revoke_session(&self, user_id: u64, service: &str) {
        let connection = self.connection.lock().unwrap();

        connection
            .execute(
                "DELETE FROM sessions WHERE user_id = ?1 AND service = ?2",
                (user_id, service),
            )
            .unwrap();

        tracing::info!("Revoked {service} for {user_id}!");
    }

    /// Deletes the given user and also scrubs their service accounts.
    pub fn delete_user(&self, user_id: u64) {
        let connection = self.connection.lock().unwrap();

        // delete from users table
        connection
            .execute("DELETE FROM users WHERE id = ?1", (user_id,))
            .unwrap();

        // delete from service accounts table
        connection
            .execute(
                "DELETE FROM service_accounts WHERE user_id = ?1",
                (user_id,),
            )
            .unwrap();

        // delete from sessions table
        connection
            .execute("DELETE FROM sessions WHERE user_id = ?1", (user_id,))
            .unwrap();

        tracing::info!("Deleted {user_id}!");
    }

    /// Grabs basic information about every user in the database.
    pub fn get_users(&self) -> Vec<User> {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT id, username FROM users")
            .unwrap();

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

    /// Returns the max expansion level for a given service account.
    pub fn get_max_expansion(&self, service_account_id: u64) -> Option<u8> {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT max_ex FROM service_accounts WHERE id = ?1")
            .unwrap();
        stmt.query_row((service_account_id,), |row| row.get(0))
            .ok()?
    }

    /// Returns the max expansion level for this user. This takes the highest expansion level from all service accounts.
    pub fn get_user_max_expansion(&self, user_id: u64) -> Option<u8> {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT MAX(max_ex) FROM service_accounts WHERE user_id = ?1")
            .unwrap();
        stmt.query_row((user_id,), |row| row.get(0)).ok()?
    }
}
