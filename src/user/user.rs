use std::sync::Arc;

use reqwest::Url;
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::{
    Session, auth::auth::GogTokenResponse, constants::constants::GOG_EMBED_URL,
    session::session::SessionError,
};

pub struct User {
    session: Session,
    token: Arc<Mutex<GogTokenResponse>>,
    owned_games: Option<Vec<u64>>,
}

impl User {
    pub fn new(session: Session, token: &GogTokenResponse) -> Self {
        User {
            session,
            token: Arc::new(Mutex::new(token.clone())),
            owned_games: None,
        }
    }
    pub async fn get_owned_games(&mut self) -> Result<Vec<u64>, SessionError> {
        let token = { self.token.lock().await.clone() };
        let url = Url::parse(&format!("{}/user/data/games", GOG_EMBED_URL)).unwrap();

        let games = self
            .session
            .get_json::<OwnedGamesResponse>(url, Some(&token.access_token), false)
            .await?;

        self.owned_games = Some(games.owned.clone());
        Ok(games.owned)
    }
}

#[derive(Deserialize, Debug)]
pub struct OwnedGamesResponse {
    pub owned: Vec<u64>,
}
