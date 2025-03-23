use std::sync::Mutex;

use rand::Rng;
use rand::distr::Alphanumeric;
use rusqlite::Connection;

pub struct LoginDatabase {
    connection: Mutex<Connection>,
}

pub enum LoginError {
    WrongUsername,
    WrongPassword,
    InternalError,
}

impl LoginDatabase {
    pub fn new() -> Self {
        let connection = Connection::open("login.db").expect("Failed to open database!");

        // Create users table
        {
            let query =
                "CREATE TABLE  IF NOT EXISTS users (username TEXT PRIMARY KEY, password TEXT);";
            connection.execute(query, ()).unwrap();
        }

        // Create active sessions table
        {
            let query =
                "CREATE TABLE IF NOT EXISTS sessions (username TEXT PRIMARY KEY, sid TEXT);";
            connection.execute(query, ()).unwrap();
        }

        Self {
            connection: Mutex::new(connection),
        }
    }

    /// Adds a new user to the database.
    pub fn add_user(&self, username: &str, password: &str) {
        let connection = self.connection.lock().unwrap();

        tracing::info!("Adding user with username {username}");

        let query = "INSERT INTO users VALUES (?1, ?2);";
        connection
            .execute(query, (username, password))
            .expect("Failed to write user to database!");
    }

    /// Login as user, returns a session id.
    pub fn login_user(&self, username: &str, password: &str) -> Result<String, LoginError> {
        let selected_row: Result<(String, String), rusqlite::Error>;

        tracing::info!("Finding user with username {username}");

        {
            let connection = self.connection.lock().unwrap();

            let mut stmt = connection
                .prepare("SELECT username, password FROM users WHERE username = ?1")
                .map_err(|_err| LoginError::WrongUsername)?;
            selected_row = stmt.query_row((username,), |row| Ok((row.get(0)?, row.get(1)?)));
        }

        if let Ok((_user, their_password)) = selected_row {
            if their_password == password {
                return self
                    .create_session(username)
                    .ok_or(LoginError::InternalError);
            } else {
                return Err(LoginError::WrongPassword);
            }
        }

        Err(LoginError::WrongUsername)
    }

    fn generate_sid() -> String {
        let random_id: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(56)
            .map(char::from)
            .collect();
        random_id.to_lowercase()
    }

    /// Create a new session for user, which replaces the last one (if any)
    pub fn create_session(&self, username: &str) -> Option<String> {
        let connection = self.connection.lock().unwrap();

        let sid = Self::generate_sid();

        connection
            .execute(
                "INSERT OR REPLACE INTO sessions VALUES (?1, ?2);",
                (username, &sid),
            )
            .ok()?;

        tracing::info!("Created new session for {username}: {sid}");

        Some(sid)
    }

    /// Checks if there is a valid session for a given id
    pub fn check_session(&self, sid: &str) -> bool {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT username, sid FROM sessions WHERE sid = ?1")
            .ok()
            .unwrap();
        let selected_row: Result<(String, String), rusqlite::Error> =
            stmt.query_row((sid,), |row| Ok((row.get(0)?, row.get(1)?)));

        selected_row.is_ok()
    }
}
