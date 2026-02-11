use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Represents a saved user session that can be restored on next launch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub user_id: String,
    pub display_name: String,
    pub homeserver: String,
    pub access_token: String,
    pub device_id: String,
}

/// Manages persistent session storage in `~/.gamechat/sessions.json`.
pub struct SessionManager;

impl SessionManager {
    /// Get the path to the sessions file.
    fn sessions_path() -> Result<PathBuf> {
        let data_dir = dirs::data_local_dir()
            .or_else(|| dirs::home_dir())
            .context("Could not determine home directory")?;

        let app_dir = data_dir.join(".gamechat");
        if !app_dir.exists() {
            fs::create_dir_all(&app_dir).context("Failed to create .gamechat directory")?;
        }

        Ok(app_dir.join("sessions.json"))
    }

    /// Load all saved sessions from disk.
    pub fn load_sessions() -> Result<Vec<Session>> {
        let path = Self::sessions_path()?;
        if !path.exists() {
            return Ok(Vec::new());
        }

        let data = fs::read_to_string(&path).context("Failed to read sessions file")?;
        let sessions: Vec<Session> =
            serde_json::from_str(&data).context("Failed to parse sessions file")?;
        Ok(sessions)
    }

    /// Save a session. If a session with the same user_id exists, it is replaced.
    pub fn save_session(session: Session) -> Result<()> {
        let mut sessions = Self::load_sessions().unwrap_or_default();

        // Replace existing session for this user, or add new
        if let Some(existing) = sessions.iter_mut().find(|s| s.user_id == session.user_id) {
            *existing = session;
        } else {
            sessions.push(session);
        }

        let path = Self::sessions_path()?;
        let data = serde_json::to_string_pretty(&sessions)?;
        fs::write(&path, data).context("Failed to write sessions file")?;
        Ok(())
    }

    /// Delete a session by user_id.
    pub fn delete_session(user_id: &str) -> Result<()> {
        let mut sessions = Self::load_sessions().unwrap_or_default();
        sessions.retain(|s| s.user_id != user_id);

        let path = Self::sessions_path()?;
        let data = serde_json::to_string_pretty(&sessions)?;
        fs::write(&path, data).context("Failed to write sessions file")?;
        Ok(())
    }

    /// Get all saved sessions for the profile switcher.
    pub fn get_remembered_profiles() -> Vec<Session> {
        Self::load_sessions().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_serialization() {
        let session = Session {
            user_id: "@test:matrix.org".to_string(),
            display_name: "TestUser".to_string(),
            homeserver: "https://matrix.org".to_string(),
            access_token: "syt_token_123".to_string(),
            device_id: "DEVICEABC".to_string(),
        };

        let json = serde_json::to_string(&session).unwrap();
        let parsed: Session = serde_json::from_str(&json).unwrap();

        assert_eq!(session.user_id, parsed.user_id);
        assert_eq!(session.display_name, parsed.display_name);
        assert_eq!(session.access_token, parsed.access_token);
    }
}
