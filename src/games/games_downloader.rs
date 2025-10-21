use std::sync::Arc;

use chrono::{DateTime, FixedOffset};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc::UnboundedSender};

use crate::{
    Session,
    auth::auth::GogTokenResponse,
    constants::constants::{GOG_CDN_URL, GOG_CONTENT_SYSTEM_URL, GOG_DB_URL},
    session::session::SessionError,
};

#[derive(Clone)]
pub struct GamesDownloader {
    session: Session,
    token: Arc<Mutex<GogTokenResponse>>,
}

impl GamesDownloader {
    pub fn new(session: &Session, token: &GogTokenResponse) -> Self {
        GamesDownloader {
            session: session.clone(),
            token: Arc::new(Mutex::new(token.clone())),
        }
    }
    pub async fn download_chunk(
        &self,
        cdns: &Vec<Cdn>,
        chunk_hash: &str,
        tx: UnboundedSender<i32>,
        is_gog_depot: bool,
    ) -> Result<(), SessionError> {
        for cdn in cdns {
            let url = {
                if is_gog_depot {
                    cdn.parse_url_redist(chunk_hash)
                } else {
                    cdn.parse_url(chunk_hash)
                }
            };

            match self
                .session
                .download_chunk(Url::parse(&url).unwrap(), |i| {
                    tx.send(i).unwrap();
                })
                .await
            {
                Ok(_) => return Ok(()),
                Err(e) => {
                    println!("Error: {}, trying again", e);
                    println!("Url: {}", url);
                    continue;
                }
            }
        }
        Err(SessionError::DownloadError(
            "All CDN attempts failed".to_string(),
        ))
    }
    pub async fn get_game_details(&self, game_id: u64) -> Result<GogDbGameDetails, SessionError> {
        let url = Url::parse(&format!(
            "{}/data/products/{}/product.json",
            GOG_DB_URL, game_id
        ))
        .unwrap();
        let response = self
            .session
            .get_json::<GogDbGameDetails>(url, None, false)
            .await?;

        Ok(response)
    }
    pub async fn get_builds_data(&self, game_id: u64) -> Result<GameBuildsData, SessionError> {
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
    pub async fn get_depot_information(&self, depot: &Depots) -> Result<DepotInfo, SessionError> {
        let url = Url::parse(&format!(
            "{}/content-system/v2/meta/{}/{}/{}",
            GOG_CDN_URL,
            &depot.manifest[0..2],
            &depot.manifest[2..4],
            depot.manifest
        ))
        .unwrap();

        let mut depot_info = self
            .session
            .get_json::<DepotInfo>(url, None, true)
            .await
            .unwrap();
        depot_info.set_is_gog_depot(depot.is_gog_depot.unwrap_or(false));
        Ok(depot_info)
    }
    pub async fn get_secure_links(
        &self,
        game_id: u64,
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

#[derive(Debug, Deserialize, Clone)]
pub struct GogDbGameDetails {
    pub title: Option<String>,
    pub image_boxart: Option<String>,
    #[serde(rename = "type")]
    pub product_type: Option<String>,
    pub game_id: Option<u64>,
}

impl GogDbGameDetails {
    pub fn set_id(&mut self, id: u64) {
        self.game_id = Some(id);
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct SecureLinksResponse {
    pub product_id: u64,
    pub urls: Vec<Cdn>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CdnUrlParams {
    pub base_url: String,
    pub path: String,
    pub token: String,
    pub expires_at: Option<u64>,
    pub dirs: Option<u64>,
    pub ttl: Option<u64>,
    pub source: Option<String>,
    pub gog_token: Option<String>,
    pub l: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Cdn {
    pub url_format: String,
    pub parameters: CdnUrlParams,
    pub priority: u64,
}
impl Cdn {
    pub fn parse_url_redist(&self, chunk_hash: &str) -> String {
        let url = format!(
            "https://gog-cdn-fastly.gog.com/content-system/v2/dependencies/store/{}/{}/{}",
            &chunk_hash[0..2],
            &chunk_hash[2..4],
            chunk_hash
        );
        url
    }
    pub fn parse_url(&self, chunk_hash: &str) -> String {
        let mut url = self.url_format.clone();
        url = url.replace("{path}", &self.parameters.path);
        url = url.replace("{token}", &self.parameters.token);
        url = url.replace("{base_url}", &self.parameters.base_url);

        if let Some(expires_at) = self.parameters.expires_at {
            url = url.replace("{expires_at}", &expires_at.to_string());
        }
        if let Some(dirs) = self.parameters.dirs {
            url = url.replace("{dirs}", &dirs.to_string());
        }
        if let Some(ttl) = self.parameters.ttl {
            url = url.replace("{ttl}", &ttl.to_string());
        }
        if let Some(source) = &self.parameters.source {
            url = url.replace("{source}", source);
        }
        if let Some(gog_token) = &self.parameters.gog_token {
            url = url.replace("{gog_token}", gog_token);
        }
        if let Some(l) = &self.parameters.l {
            url = url.replace("{l}", l);
        }
        let galaxy_path = format!(
            "/{}/{}/{}",
            &chunk_hash[0..2],
            &chunk_hash[2..4],
            chunk_hash
        );
        url = format!("{}{}", url, galaxy_path);

        url
    }
}

#[derive(Deserialize, Debug)]
pub struct DepotInfo {
    pub depot: Item,
    pub version: u64,
    pub is_gog_depot: Option<bool>,
}
impl DepotInfo {
    pub fn set_is_gog_depot(&mut self, is_gog_depot: bool) {
        self.is_gog_depot = Some(is_gog_depot);
    }
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
    pub chunks: Option<Vec<Chunk>>,
    pub sha256: Option<String>,
    pub is_gog_depot: Option<bool>,
}
impl DepotFile {
    pub fn set_is_gog_depot(&mut self, is_gog_depot: bool) {
        self.is_gog_depot = Some(is_gog_depot);
    }
}

impl DepotFile {
    pub fn get_size(&self) -> u64 {
        self.chunks
            .as_ref()
            .map_or(0, |chunks| chunks.iter().map(|chunk| chunk.size).sum())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Chunk {
    pub md5: String,
    pub size: u64,
    #[serde(rename = "compressedMd5")]
    pub compressed_md5: String,
    #[serde(rename = "compressedSize")]
    pub compressed_size: u64,
    pub order: Option<i32>,
}

impl Chunk {
    pub fn set_order(&mut self, order: i32) {
        self.order = Some(order);
    }
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
    #[serde(rename = "isGogDepot")]
    pub is_gog_depot: Option<bool>,
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
