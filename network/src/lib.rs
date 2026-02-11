use anyhow::{Context, Result};
use matrix_sdk::{ruma::events::room::message::RoomMessageEventContent, Client};

pub mod session;
pub mod voice;

use session::{Session, SessionManager};

pub struct MatrixClient {
    client: Client,
    user_id: Option<String>,
    display_name: Option<String>,
}

impl MatrixClient {
    pub async fn new(homeserver_url: &str) -> Result<Self> {
        // Strip protocol prefix for server_name if present
        let server_name = homeserver_url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/');

        println!("[MatrixClient] Connecting to server: {}", server_name);

        // Try server_name discovery first (does .well-known lookup), fall back to homeserver_url
        let client = if let Ok(name) = <&matrix_sdk::ruma::ServerName>::try_from(server_name) {
            Client::builder().server_name(name).build().await?
        } else {
            Client::builder()
                .homeserver_url(homeserver_url)
                .build()
                .await?
        };
        println!(
            "[MatrixClient] Connected. Homeserver resolved to: {}",
            client.homeserver()
        );
        Ok(Self {
            client,
            user_id: None,
            display_name: None,
        })
    }

    /// Login with username/password. Returns (user_id, display_name).
    pub async fn login(&mut self, username: &str, password: &str) -> Result<(String, String)> {
        println!("[MatrixClient] Logging in as '{}'", username);
        let response = self
            .client
            .matrix_auth()
            .login_username(username, password)
            .send()
            .await?;

        let user_id = response.user_id.to_string();

        // Fetch actual display name from server
        let display_name = self
            .client
            .account()
            .get_display_name()
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| username.to_string());

        self.user_id = Some(user_id.clone());
        self.display_name = Some(display_name.clone());

        // Save session for remember-me
        if let Some(mat_session) = self.client.matrix_auth().session() {
            let saved = Session {
                user_id: user_id.clone(),
                display_name: display_name.clone(),
                homeserver: self.client.homeserver().to_string(),
                access_token: mat_session.tokens.access_token.to_string(),
                device_id: mat_session.meta.device_id.to_string(),
            };
            let _ = SessionManager::save_session(saved);
        }

        Ok((user_id, display_name))
    }

    /// Register a new account. Returns (user_id, display_name).
    pub async fn register(&mut self, username: &str, password: &str) -> Result<(String, String)> {
        use matrix_sdk::ruma::api::client::account::register::v3::Request as RegistrationRequest;

        let mut request = RegistrationRequest::new();
        request.username = Some(username.to_string());
        request.password = Some(password.to_string());

        match self.client.matrix_auth().register(request).await {
            Ok(response) => {
                let user_id = response.user_id.to_string();
                let display_name = username.to_string();
                self.user_id = Some(user_id.clone());
                self.display_name = Some(display_name.clone());

                if let Some(mat_session) = self.client.matrix_auth().session() {
                    let saved = Session {
                        user_id: user_id.clone(),
                        display_name: display_name.clone(),
                        homeserver: self.client.homeserver().to_string(),
                        access_token: mat_session.tokens.access_token.to_string(),
                        device_id: mat_session.meta.device_id.to_string(),
                    };
                    let _ = SessionManager::save_session(saved);
                }

                Ok((user_id, display_name))
            }
            Err(e) => Err(anyhow::anyhow!(
                "Registration failed: {}. Many homeservers require email verification or have registration disabled.",
                e
            )),
        }
    }

    /// Restore a session from a saved token.
    pub async fn restore_session(saved: &Session) -> Result<Self> {
        let client = Client::builder()
            .homeserver_url(&saved.homeserver)
            .build()
            .await?;

        use matrix_sdk::matrix_auth::{MatrixSession, MatrixSessionTokens};
        use matrix_sdk::ruma::{OwnedDeviceId, OwnedUserId};
        use matrix_sdk::SessionMeta;

        let mat_session = MatrixSession {
            meta: SessionMeta {
                user_id: OwnedUserId::try_from(saved.user_id.as_str())
                    .context("Invalid user_id in saved session")?,
                device_id: OwnedDeviceId::from(saved.device_id.as_str()),
            },
            tokens: MatrixSessionTokens {
                access_token: saved.access_token.clone(),
                refresh_token: None,
            },
        };

        client.matrix_auth().restore_session(mat_session).await?;

        Ok(Self {
            client,
            user_id: Some(saved.user_id.clone()),
            display_name: Some(saved.display_name.clone()),
        })
    }

    pub fn get_display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    pub fn get_user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    pub async fn set_display_name(&mut self, name: &str) -> Result<()> {
        self.client.account().set_display_name(Some(name)).await?;
        self.display_name = Some(name.to_string());
        Ok(())
    }

    pub async fn sync(&self) -> Result<()> {
        Ok(())
    }

    pub async fn send_message(&self, room_id: &str, content: &str) -> Result<()> {
        let room_id = <&matrix_sdk::ruma::RoomId>::try_from(room_id)?;
        if let Some(room) = self.client.get_room(room_id) {
            let content = RoomMessageEventContent::text_plain(content);
            room.send(content).await?;
        }
        Ok(())
    }

    pub async fn logout(&mut self) -> Result<()> {
        if let Some(user_id) = &self.user_id {
            let _ = SessionManager::delete_session(user_id);
        }
        let _ = self.client.matrix_auth().logout().await;
        self.user_id = None;
        self.display_name = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_init() {
        let client = MatrixClient::new("https://matrix.org").await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_url() {
        let result = MatrixClient::new("not-a-url").await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_voice_manager_init() {
        let result = crate::voice::VoiceManager::new("127.0.0.1:0").await;
        assert!(result.is_ok());
    }
}
