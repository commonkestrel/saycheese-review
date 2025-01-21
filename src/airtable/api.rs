use std::collections::HashMap;

use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;
use url::Url;

const AIRTABLE_API_BASE: &str = "https://api.airtable.com/v0";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RecordId(String);

pub struct ListRecords {
    /// Airtable base ID
    base: String,
    /// Base table name or ID
    table: String,
    /// The maximum total number of records that will be returned in your requests.
    /// If this value is larger than `100`, multiple API requests will be required.
    /// Defaults to unlimited.
    max_records: Option<usize>,
    /// The name or ID of a view in the table.
    /// If set, only the records in that view will be returned.
    /// The records will be sorted according to the order of the view
    /// unless the `sort` parameter is included, which overrides that order.
    /// Fields hidden in this view will be returned in the results.
    /// To only return a subset of fields, use the fields parameter.
    view: Option<String>,
    sort: Option<Sort>,
    filter_by_formula: Option<String>,
    fields: Option<Vec<String>>,
}

impl ListRecords {
    pub fn new(base: String, table: String) -> ListRecords {
        ListRecords {
            base,
            table,
            max_records: None,
            view: None,
            sort: None,
            filter_by_formula: None,
            fields: None,
        }
    }

    pub fn max_records(&mut self, max: usize) -> &mut Self {
        self.max_records = Some(max);
        self
    }

    pub fn with_max_records(mut self, max: usize) -> Self {
        self.max_records(max);
        self
    }

    pub fn view(&mut self, view: String) -> &mut Self {
        self.view = Some(view);
        self
    }

    pub fn with_view(mut self, view: String) -> Self {
        self.view(view);
        self
    }

    pub fn sort(&mut self, field: String, direction: Direction) -> &mut Self {
        self.sort = Some(Sort { field, direction });
        self
    }

    pub fn with_sort(mut self, field: String, direction: Direction) -> Self {
        self.sort(field, direction);
        self
    }

    pub fn filter_by_formula(&mut self, formula: String) -> &mut Self {
        self.filter_by_formula = Some(formula);
        self
    }

    pub fn with_filter_by_formula(mut self, formula: String) -> Self {
        self.filter_by_formula(formula);
        self
    }

    pub fn fields(&mut self, fields: Vec<String>) -> &mut Self {
        self.fields = Some(fields);
        self
    }

    pub fn with_fields(mut self, fields: Vec<String>) -> Self {
        self.fields(fields);
        self
    }

    pub async fn request<T>(self, key: &str) -> Result<Vec<Record<T>>, ApiError>
    where
        T: DeserializeOwned,
    {
        // create formatted base url for the given base and table
        let mut url = Url::parse(&format!("{AIRTABLE_API_BASE}/{}/{}", self.base, self.table))?;
        let mut pairs = url.query_pairs_mut();

        if let Some(view) = self.view {
            pairs.append_pair("view", &view);
        }

        if let Some(sort) = self.sort {
            pairs.append_pair("sort[0][field]", &sort.field);
            pairs.append_pair(
                "sort[0][direction]",
                match sort.direction {
                    Direction::Ascending => "asc",
                    Direction::Descending => "desc",
                },
            );
        }

        if let Some(formula) = self.filter_by_formula {
            pairs.append_pair("filterByFormula", &formula);
        }

        if let Some(fields) = self.fields {
            for field in fields {
                pairs.append_pair("fields[]", &field);
            }
        }

        // get rid of `pairs` so we can use `url` later
        std::mem::drop(pairs);

        let mut records = Vec::new();
        let mut max_records = self.max_records;
        let mut offset: Option<String> = None;
        let client = reqwest::Client::new();

        loop {
            let mut endpoint = url.clone();

            if let Some(max) = max_records.as_mut() {
                if *max == 0 {
                    break;
                }

                if *max > 100 {
                    *max -= 100;
                } else {
                    endpoint
                        .query_pairs_mut()
                        .append_pair("maxRecords", &max.to_string());
                    *max = 0;
                }
            }

            if let Some(off) = offset {
                endpoint.query_pairs_mut().append_pair("offset", &off);
            }

            let res = client
                .get(endpoint.to_string())
                .header("Authorization", format!("Bearer {}", key))
                .send()
                .await?;

            let status = res.status();
            if !status.is_success() {
                return Err(ApiError::Api {
                    status,
                    message: res.text().await?,
                });
            }

            let content: ListResponse<T> = res.json().await?;
            records.extend(content.records);

            if let Some(off) = content.offset {
                offset = Some(off);
                continue;
            }

            break;
        }

        Ok(records)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct ListResponse<T> {
    records: Vec<Record<T>>,
    offset: Option<String>,
}

pub async fn update_record<T>(
    key: &str,
    base: &str,
    table: &str,
    id: &RecordId,
    data: T,
    typecast: bool,
) -> Result<Record<T>, ApiError>
where
    T: Serialize + DeserializeOwned,
{
    let url = format!("{AIRTABLE_API_BASE}/{base}/{table}/{}", id.0);
    let client = reqwest::Client::new();

    let mut map = HashMap::new();
    map.insert("typecast", serde_json::Value::Bool(typecast));
    map.insert("fields", serde_json::to_value(data)?);

    let res = client
        .patch(url)
        .header("Authorization", format!("Bearer {}", key))
        .json(&map)
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        return Err(ApiError::Api {
            status,
            message: res.text().await?,
        });
    }

    let record: Record<T> = res.json().await?;

    Ok(record)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Record<T> {
    id: RecordId,
    #[serde(rename = "createdTime")]
    created_time: DateTime<Utc>,
    fields: T,
}

impl<T> Record<T> {
    pub fn id(&self) -> &RecordId {
        &self.id
    }

    pub fn created_time(&self) -> DateTime<Utc> {
        self.created_time
    }

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

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("unable to parse endpoint url")]
    Url(#[from] url::ParseError),
    #[error("unable to reach API endpoint")]
    Http(#[from] reqwest::Error),
    #[error("unable to encode or decode JSON data")]
    Json(#[from] serde_json::Error),
    #[error("API request failed")]
    Api { status: StatusCode, message: String },
}

pub enum ApiRequest {
    /// Get record
    GetRecord,
    /// Update multiple records
    UpdateRecords,
    /// Update record
    UpdateRecord,
    /// Create records
    CreateRecord,
    /// Delete multiple records
    DeleteRecords,
    /// Delete record
    DeleteRecord,
    /// Upload attachment
    Attachment,
    /// Update field
    UpdateField,
    /// Create field
    CreateField,
    /// List comments
    ListComments,
    /// Update comment
    UpdateComment,
    /// Create comment
    CreateComment,
    /// Delete comment
    DeleteComment,
}

struct Sort {
    field: String,
    direction: Direction,
}

pub enum Direction {
    Ascending,
    Descending,
}
