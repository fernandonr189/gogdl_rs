use std::sync::Arc;

use chrono::{DateTime, FixedOffset};
use reqwest::Url;
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::{
    Session,
    auth::auth::GogTokenResponse,
    constants::constants::{GOG_CDN_URL, GOG_CONTENT_SYSTEM_URL},
    session::session::{self, SessionError},
};

pub struct GamesDownloader {
    session: Session,
    token: Arc<Mutex<GogTokenResponse>>,
}

impl GamesDownloader {
    pub fn new(session: Session, token: &GogTokenResponse) -> Self {
        GamesDownloader {
            session,
            token: Arc::new(Mutex::new(token.clone())),
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
    pub async fn get_build_metadata(
        &self,
        build_link: &str,
    ) -> Result<BuildMetadata, SessionError> {
        let url = Url::parse(build_link).unwrap();

        let metadata = self
            .session
            .get_json::<BuildMetadata>(url, None, true)
            .await?;
        Ok(metadata)
    }
    pub async fn get_depot_information(
        &self,
        depot_manifest: &str,
    ) -> Result<DepotInfo, SessionError> {
        let url = Url::parse(&format!(
            "{}/content-system/v2/meta/{}/{}/{}",
            GOG_CDN_URL,
            &depot_manifest[0..2],
            &depot_manifest[2..4],
            depot_manifest
        ))
        .unwrap();

        let depot_info = self.session.get_json::<DepotInfo>(url, None, true).await?;
        Ok(depot_info)
    }
    pub async fn get_secure_links(
        &self,
        game_id: &str,
    ) -> Result<SecureLinksResponse, SessionError> {
        let url = Url::parse(&format!(
            "{}/products/{}/secure_link?generation=2&_version=2&path=/",
            GOG_CONTENT_SYSTEM_URL, game_id
        ))
        .unwrap();

        let token = { self.token.lock().await.clone() };

        let secure_link_response = self
            .session
            .get_json::<SecureLinksResponse>(url, Some(&token.access_token), false)
            .await?;
        Ok(secure_link_response)
    }
}

#[derive(Deserialize, Debug)]
pub struct SecureLinksResponse {
    pub product_id: u64,
    pub urls: Vec<Cdn>,
}

#[derive(Deserialize, Debug)]
pub struct Cdn {
    pub url_format: String,
    pub parameters: CdnUrlParams,
    pub priority: u64,
}

#[derive(Deserialize, Debug)]
pub struct CdnUrlParams {
    pub base_url: String,
    pub path: String,
    pub token: String,
}

#[derive(Deserialize, Debug)]
pub struct DepotInfo {
    pub depot: Item,
    pub version: u64,
}

#[derive(Deserialize, Debug)]
pub struct Item {
    pub items: Vec<DepotFile>,
}

#[derive(Deserialize, Debug)]
pub struct DepotFile {
    pub path: String,
    #[serde(rename = "type")]
    pub file_type: String,
}

#[derive(Deserialize, Debug)]
pub struct BuildMetadata {
    pub version: u64,
    #[serde(rename = "baseProductId")]
    pub base_product_id: String,
    pub dependencies: Vec<String>,
    pub depots: Vec<Depots>,
}

#[derive(Deserialize, Debug)]
pub struct Depots {
    pub manifest: String,
    pub size: u64,
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
    pub date_published: String,
}

impl GameBuild {
    pub fn get_date(&self) -> Result<DateTime<FixedOffset>, chrono::ParseError> {
        self.date_published.parse::<DateTime<FixedOffset>>()
    }
}
