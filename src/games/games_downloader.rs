use std::sync::Arc;

use reqwest::Url;
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::{
    Session, auth::auth::GogTokenResponse, constants::constants::GOG_CONTENT_SYSTEM_URL,
    session::session::SessionError,
};

pub struct GamesDownloader {
    session: Session,
    _token: Arc<Mutex<GogTokenResponse>>,
}

impl GamesDownloader {
    pub fn new(session: Session, token: &GogTokenResponse) -> Self {
        GamesDownloader {
            session,
            _token: Arc::new(Mutex::new(token.clone())),
        }
    }
    pub async fn get_builds_data(&self, game_id: &str) -> Result<GameBuildsData, SessionError> {
        let url = Url::parse(&format!(
            "{}/products/{}/os/windows/builds?generation=2",
            GOG_CONTENT_SYSTEM_URL, game_id
        ))
        .unwrap();

        let game_builds_data = self
            .session
            .get_json::<GameBuildsData>(url, None, false)
            .await?;

        Ok(game_builds_data)
    }
}

#[derive(Debug, Deserialize)]
pub struct GameBuildsData {
    pub total_count: u64,
    pub count: u64,
    pub items: Vec<GameBuild>,
}

#[derive(Debug, Deserialize)]
pub struct GameBuild {
    pub build_id: String,
    pub version_name: String,
    pub generation: u64,
    pub link: String,
}
