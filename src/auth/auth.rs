use std::{error::Error, fmt::Display, sync::Arc};

use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    constants::constants::{
        GOG_AUTH_GRANT_TYPE, GOG_AUTH_URL, GOG_CLIENT_ID, GOG_CLIENT_SECRET, GOG_LOGIN_URL,
        GOG_REDIRECT_URI, GOG_REFRESH_GRANT_TYPE,
    },
    session::session::{Session, SessionError},
};

#[derive(Clone)]
pub struct Auth {
    session: Session,
    pub token: Option<Arc<Mutex<GogTokenResponse>>>,
}

impl Auth {
    pub fn new(session: &Session) -> Self {
        Auth {
            session: session.clone(),
            token: None,
        }
    }
    pub async fn get_token(&self) -> Result<GogTokenResponse, AuthError> {
        let token = self.token.as_ref().ok_or(AuthError::NoAuthToken)?;
        let token = token.lock().await;
        Ok(token.clone())
    }
    pub fn recover_session(&mut self, token: &GogTokenResponse) {
        self.token = Some(Arc::new(Mutex::new(token.clone())));
    }
    pub async fn login(&mut self, login_code: &str) -> Result<(), SessionError> {
        let mut url = Url::parse(&format!("{}/token", GOG_AUTH_URL)).unwrap();
        url.query_pairs_mut()
            .append_pair("client_id", GOG_CLIENT_ID);
        url.query_pairs_mut()
            .append_pair("client_secret", GOG_CLIENT_SECRET);
        url.query_pairs_mut()
            .append_pair("grant_type", GOG_AUTH_GRANT_TYPE);
        url.query_pairs_mut().append_pair("code", login_code);
        url.query_pairs_mut()
            .append_pair("redirect_uri", GOG_REDIRECT_URI);

        let response = self
            .session
            .get_json::<GogTokenResponse>(url, None, false)
            .await?;

        self.token = Some(Arc::new(Mutex::new(response)));
        Ok(())
    }
    pub async fn refresh_token(&mut self) -> Result<(), AuthError> {
        let refresh_token = {
            let token = match self.token.as_ref() {
                Some(token) => token,
                None => return Err(AuthError::NoAuthToken),
            };
            token.lock().await.refresh_token.clone()
        };

        let mut url = Url::parse(&format!("{}/token", GOG_AUTH_URL)).unwrap();
        url.query_pairs_mut()
            .append_pair("client_id", GOG_CLIENT_ID);
        url.query_pairs_mut()
            .append_pair("client_secret", GOG_CLIENT_SECRET);
        url.query_pairs_mut()
            .append_pair("grant_type", GOG_REFRESH_GRANT_TYPE);
        url.query_pairs_mut()
            .append_pair("refresh_token", &refresh_token);

        let response = match self
            .session
            .get_json::<GogTokenResponse>(url, None, false)
            .await
        {
            Ok(token) => token,
            Err(err) => return Err(AuthError::RefreshTokenError(err.to_string())),
        };
        self.token = Some(Arc::new(Mutex::new(response)));
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GogTokenResponse {
    pub expires_in: i64,
    pub scope: String,
    pub token_type: String,
    pub access_token: String,
    pub user_id: String,
    pub refresh_token: String,
    pub session_id: String,
}
#[derive(Debug)]
pub enum AuthError {
    NoAuthToken,
    RefreshTokenError(String),
}

impl Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::NoAuthToken => write!(f, "No auth token, have you logged in?"),
            AuthError::RefreshTokenError(err) => write!(f, "Failed to refresh token: {}", err),
        }
    }
}

impl Error for AuthError {}
