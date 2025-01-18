use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RecordId(String);

pub enum ApiRequest {
    /// List records
    ListRecords {
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
    },
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
