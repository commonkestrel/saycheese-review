pub mod api;
pub mod types;

use std::{collections::HashMap, io, str::FromStr};

use reqwest::StatusCode;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::Url;

const AIRTABLE_URL_BASE: &str = "https://api.airtable.com/v0/";

#[derive(Debug, Clone)]
pub struct Base {
    key: String,
    destination: Url,
}

impl Base {
    pub fn new(key: String, base: &str, table: &str, view: &str) -> Self {
        let mut uri =
            Url::from_str(AIRTABLE_URL_BASE).expect("airtable url base should be parsable");

        uri = uri
            .join(&(base.to_owned() + "/"))
            .expect("should be able to join base ID to destination")
            .join(table)
            .expect("should be able to join table to destination");

        uri.set_query(Some(&format!("view={view}")));

        Self {
            key,
            destination: uri,
        }
    }

    pub async fn query(&self) -> Result<serde_json::Value, RequestError> {
        let client = reqwest::Client::new();
        let res = client
            .get(self.destination.to_string())
            .header("Authorization", format!("Bearer {}", self.key))
            .send()
            .await?;

        log::debug!("sent request to {}", self.destination.to_string());

        let status = res.status();
        let body = match res.json().await {
            Ok(body) => body,
            Err(err) => return Err(RequestError::Body(err)),
        };

        if status != StatusCode::OK {
            return Err(RequestError::Airtable { status, body });
        }

        return Ok(body);
    }

    pub async fn read_records<T>(&self) -> Result<Vec<Record<T>>, RequestError>
    where
        T: DeserializeOwned,
    {
        let res = self.query().await?;
        let parsed: Response<T> = serde_json::from_value(res)?;
        Ok(parsed.records)
    }

    pub async fn update_records<T>(&self, records: &[Record<T>]) -> Result<(), serde_json::Error>
    where
        T: Serialize,
    {
        let fields = serde_json::to_value(records)?;
        let mut record_map = HashMap::new();
        record_map.insert("fields", fields);

        Ok(())
    }
}

#[derive(Deserialize)]
struct Response<T> {
    records: Vec<Record<T>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Record<T> {
    id: String,
    #[serde(skip_serializing, rename = "createdTime")]
    created_time: String,
    fields: T,
}

impl<T> Record<T> {
    pub fn fields(&self) -> &T {
        &self.fields
    }

    pub fn fields_mut(&mut self) -> &mut T {
        &mut self.fields
    }

    pub fn into_fields(self) -> T {
        self.fields
    }
}

#[derive(Debug)]
pub enum RequestError {
    Io(io::Error),
    Https(reqwest::Error),
    Body(reqwest::Error),
    Json(serde_json::Error),
    Airtable {
        status: StatusCode,
        body: serde_json::Value,
    },
}

impl From<io::Error> for RequestError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<reqwest::Error> for RequestError {
    fn from(value: reqwest::Error) -> Self {
        Self::Https(value)
    }
}

impl From<serde_json::Error> for RequestError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    id: String,
    width: usize,
    height: usize,
    url: String,
    filename: String,
    size: usize,
    #[serde(rename = "type")]
    ty: String,
    thumbnails: Thumbnails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thumbnails {
    small: Thumbnail,
    large: Thumbnail,
    full: Thumbnail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thumbnail {
    url: String,
    width: usize,
    height: usize,
}
