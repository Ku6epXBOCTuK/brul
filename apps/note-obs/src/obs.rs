//! Клиент OBS WebSocket: подключение, проверка сцены и запись текста.

use std::fmt;
use std::time::Duration;

use obws::Client;
use obws::client::HandshakeError;
use obws::error::Error;
use obws::requests::inputs::{InputId, SetSettings};
use serde_json::json;
use tokio::time::timeout;

use crate::config::Config;

const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObsError {
    Runtime(String),
    Timeout,
    Password,
    Unreachable(String),
    Scene(String),
    Source(String),
}

impl fmt::Display for ObsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(message) => write!(formatter, "tokio runtime: {message}"),
            Self::Timeout => write!(
                formatter,
                "OBS did not answer in {} seconds",
                TIMEOUT.as_secs()
            ),
            Self::Password => formatter.write_str("OBS rejected the password"),
            Self::Unreachable(message) => write!(formatter, "OBS is unreachable: {message}"),
            Self::Scene(name) => write!(formatter, "no scene '{name}' in OBS"),
            Self::Source(message) => write!(formatter, "source: {message}"),
        }
    }
}

impl std::error::Error for ObsError {}

pub fn write_text(config: &Config, text: &str) -> Result<(), ObsError> {
    runtime()?.block_on(async {
        let client = connect(config).await?;
        let settings = json!({ "text": text });
        client
            .inputs()
            .set_settings(SetSettings {
                input: InputId::Name(&config.source),
                settings: &settings,
                overlay: Some(true),
            })
            .await
            .map_err(|error| ObsError::Source(error.to_string()))
    })
}

fn runtime() -> Result<tokio::runtime::Runtime, ObsError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| ObsError::Runtime(error.to_string()))
}

async fn connect(config: &Config) -> Result<Client, ObsError> {
    let password = Some(config.password.as_str()).filter(|password| !password.is_empty());
    let client = timeout(
        TIMEOUT,
        Client::connect(&config.host, config.port, password),
    )
    .await
    .map_err(|_| ObsError::Timeout)?
    .map_err(connect_error)?;
    let scenes = client
        .scenes()
        .list()
        .await
        .map_err(|error| ObsError::Unreachable(error.to_string()))?;
    if !scenes
        .scenes
        .iter()
        .any(|scene| scene.id.name == config.scene)
    {
        return Err(ObsError::Scene(config.scene.clone()));
    }
    Ok(client)
}

fn connect_error(error: Error) -> ObsError {
    match error {
        Error::Timeout => ObsError::Timeout,
        Error::Handshake(HandshakeError::ConnectionClosed(Some(details)))
            if is_password_error(&details.reason) =>
        {
            ObsError::Password
        }
        other => ObsError::Unreachable(other.to_string()),
    }
}

fn is_password_error(reason: &str) -> bool {
    let reason = reason.to_lowercase();
    reason.contains("password") || reason.contains("authenticat")
}

#[cfg(test)]
mod tests {
    use super::{ObsError, connect_error, is_password_error};

    #[test]
    fn a_password_reason_is_recognised() {
        assert!(is_password_error("Authentication failed."));
        assert!(is_password_error("wrong password"));
        assert!(!is_password_error("normal closure"));
    }

    #[test]
    fn a_timeout_has_its_own_message() {
        let error = connect_error(obws::error::Error::Timeout);
        assert_eq!(error, ObsError::Timeout);
        assert!(error.to_string().contains("did not answer"));
    }

    #[test]
    fn every_error_names_its_cause() {
        let messages = [
            ObsError::Timeout,
            ObsError::Password,
            ObsError::Runtime(String::from("boom")),
            ObsError::Unreachable(String::from("refused")),
            ObsError::Scene(String::from("Main")),
            ObsError::Source(String::from("no text")),
        ];
        let texts: Vec<String> = messages.iter().map(ToString::to_string).collect();
        assert!(texts[1].contains("password"));
        assert!(texts[4].contains("Main"));
        assert!(texts[5].contains("no text"));
        assert!(texts.iter().all(|text| text.is_ascii()));
    }
}
