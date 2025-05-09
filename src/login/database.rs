use std::sync::Mutex;

use rusqlite::Connection;

use crate::ipc::lobby::ServiceAccount;

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
            let query =
                "CREATE TABLE IF NOT EXISTS sessions (user_id INTEGER PRIMARY KEY, sid TEXT);";
            connection.execute(query, ()).unwrap();
        }

        // Create service accounts table
        {
            let query = "CREATE TABLE IF NOT EXISTS service_accounts (id INTEGER PRIMARY KEY, user_id INTEGER);";
            connection.execute(query, ()).unwrap();
        }

        Self {
            connection: Mutex::new(connection),
        }
    }

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

            let query = "INSERT INTO service_accounts VALUES (?1, ?2);";
            connection
                .execute(query, (Self::generate_account_id(), user_id))
                .expect("Failed to write service account to database!");
        }
    }

    /// Login as user, returns a session id.
    pub fn login_user(&self, username: &str, password: &str) -> Result<String, LoginError> {
        let selected_row: Result<(u32, String), rusqlite::Error>;

        tracing::info!("Finding user with username {username}");

        {
            let connection = self.connection.lock().unwrap();

            let mut stmt = connection
                .prepare("SELECT id, password FROM users WHERE username = ?1")
                .map_err(|_err| LoginError::WrongUsername)?;
            selected_row = stmt.query_row((username,), |row| Ok((row.get(0)?, row.get(1)?)));
        }

        if let Ok((_id, their_password)) = selected_row {
            if their_password == password {
                return self.create_session(_id).ok_or(LoginError::InternalError);
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

    /// Create a new session for user, which replaces the last one (if any)
    pub fn create_session(&self, user_id: u32) -> Option<String> {
        let connection = self.connection.lock().unwrap();

        let sid = Self::generate_sid();

        connection
            .execute(
                "INSERT OR REPLACE INTO sessions VALUES (?1, ?2);",
                (user_id, &sid),
            )
            .ok()?;

        tracing::info!("Created new session for account {user_id}: {sid}");

        Some(sid)
    }

    /// Gets the service account list
    pub fn check_session(&self, sid: &str) -> Vec<ServiceAccount> {
        let connection = self.connection.lock().unwrap();

        // get user id
        let user_id: u32;
        {
            let mut stmt = connection
                .prepare("SELECT user_id FROM sessions WHERE sid = ?1")
                .ok()
                .unwrap();
            if let Ok(found_user_id) = stmt.query_row((sid,), |row| row.get(0)) {
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
                    unk1: 0,
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
        let selected_row: Result<u32, rusqlite::Error> =
            stmt.query_row((username,), |row| row.get(0));

        selected_row.is_ok()
    }

    pub fn get_user_id(&self, sid: &str) -> u32 {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT user_id FROM sessions WHERE sid = ?1")
            .ok()
            .unwrap();
        stmt.query_row((sid,), |row| row.get(0)).unwrap()
    }

    pub fn get_username(&self, user_id: u32) -> String {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT username FROM users WHERE id = ?1")
            .ok()
            .unwrap();
        stmt.query_row((user_id,), |row| row.get(0)).unwrap()
    }

    /// TODO: only works for one
    pub fn get_service_account(&self, user_id: u32) -> u32 {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT id FROM service_accounts WHERE user_id = ?1")
            .ok()
            .unwrap();
        stmt.query_row((user_id,), |row| row.get(0)).unwrap()
    }
}
