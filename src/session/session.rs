use flate2::read::ZlibDecoder;
use futures_util::StreamExt;
use reqwest::{StatusCode, Url};
use std::{error::Error, fmt::Display, io::Read};

#[derive(Clone)]
pub struct Session {
    pub session: reqwest::Client,
}

impl Session {
    pub fn new() -> Self {
        Session {
            session: reqwest::Client::new(),
        }
    }
    pub async fn download_chunk<F>(&self, url: Url, callback: F) -> Result<(), SessionError>
    where
        F: Fn(i32),
    {
        let response = self
            .session
            .get(url)
            .send()
            .await
            .map_err(|err| SessionError::NetworkError(err.to_string()))?;

        if !response.status().is_success() {
            return Err(SessionError::NetworkError(format!(
                "Unexpected status: {}",
                response.status()
            )));
        }

        let mut stream = response.bytes_stream();
        while let Some(item) = stream.next().await {
            match item {
                Ok(chunk) => callback(chunk.len() as i32),
                Err(e) => {
                    return Err(SessionError::NetworkError(e.to_string()));
                }
            }
        }
        Ok(())
    }
    pub async fn get_json<T>(
        &self,
        url: Url,
        auth_token: Option<&str>,
        compressed: bool,
    ) -> Result<T, SessionError>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut request = self.session.get(url);
        if let Some(token) = auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        let response = match request.send().await {
            Ok(response) => response,
            Err(err) => return Err(SessionError::NetworkError(err.to_string())),
        };
        if response.status() != StatusCode::OK {
            return Err(SessionError::HttpError(response.status().as_u16()));
        }
        if compressed {
            let bytes = if let Ok(bytes) = response.bytes().await {
                bytes
            } else {
                return Err(SessionError::NetworkError(
                    "Failed to read response bytes".to_string(),
                ));
            };
            let mut d = ZlibDecoder::new(&bytes[..]);
            let mut s = String::new();
            d.read_to_string(&mut s).unwrap();
            match serde_json::from_str::<T>(&s) {
                Ok(data) => Ok(data),
                Err(err) => Err(SessionError::DeserializationError(err.to_string())),
            }
        } else {
            match response.json::<T>().await {
                Ok(data) => Ok(data),
                Err(err) => Err(SessionError::DeserializationError(err.to_string())),
            }
        }
    }
}

#[derive(Debug)]
pub enum SessionError {
    HttpError(u16),
    NetworkError(String),
    DeserializationError(String),
    DecompressionError(String),
    DownloadError(String),
}

impl Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::HttpError(status) => write!(f, "HTTP error: {}", status),
            SessionError::NetworkError(err) => write!(f, "Network error: {}", err),
            SessionError::DeserializationError(err) => write!(f, "Deserialization error: {}", err),
            SessionError::DecompressionError(err) => write!(f, "Decompression error: {}", err),
            SessionError::DownloadError(err) => write!(f, "Download error: {}", err),
        }
    }
}

impl Error for SessionError {}
