use std::fs::File;

use actix_files::{Files, NamedFile};
use actix_web::{get, middleware::Logger, post, web, App, HttpResponse, HttpServer, Responder};
use airtable::{
    api::{ListRecords, Record, RecordId},
    Attachment, Base,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::json;

mod airtable;

const AIRTABLE_API_KEY: &str = env!("AIRTABLE_API_KEY");
const AIRTABLE_BASE_ID: &str = env!("AIRTABLE_BASE_ID");
const ICON: &[u8; 76109] = include_bytes!("../static/say-cheese.png");
const EMAIL: &str = include_str!("../static/email.html");
const IMAGE_DATA_URI: &str = "data:image/png;base64,";

const SUBMISSION_TABLE: &str = "YSWS Project Submission";
const TABLE_VIEW: &str = "Grid View";

const FIELDS: [&str; 11] = [
    "project_name",
    "Code URL",
    "Screenshot",
    "Description",
    "Optional - Override Hours Spent",
    "Email",
    "qr_code",
    "gallery_attribution",
    "os",
    "architecture",
    "status",
];

// listen i didnt name the records dont blame me
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Submission {
    #[serde(rename = "project_name")]
    name: String,
    #[serde(rename = "Code URL")]
    repo_url: String,
    #[serde(rename = "Screenshot")]
    screenshot: Vec<Attachment>,
    #[serde(rename = "Description")]
    description: String,
    #[serde(rename = "Optional - Override Hours Spent")]
    hours: f32,
    #[serde(rename = "Email")]
    email: String,
    qr_code: Vec<Attachment>,
    gallery_attribution: String,
    os: String,
    architecture: String,
    #[serde(default = "default_status")]
    status: String,
}

fn default_status() -> String {
    "new".to_owned()
}

#[get("/record/{i}")]
async fn record(i: web::Path<usize>) -> impl Responder {
    let json: Vec<Record<Submission>> =
        ListRecords::new(AIRTABLE_BASE_ID.to_owned(), SUBMISSION_TABLE.to_owned())
            .with_view(TABLE_VIEW.to_owned())
            .with_fields(FIELDS.iter().map(ToString::to_string).collect())
            .request(AIRTABLE_API_KEY)
            .await
            .unwrap();

    json.get(i.into_inner()).cloned().map(web::Json)
}

#[get("/nextrecord")]
async fn next_record() -> impl Responder {
    let records: Vec<Record<Submission>> =
        ListRecords::new(AIRTABLE_BASE_ID.to_owned(), SUBMISSION_TABLE.to_owned())
            .with_view(TABLE_VIEW.to_owned())
            .with_fields(FIELDS.iter().map(ToString::to_string).collect())
            .with_filter_by_formula("status = \"new\"".to_owned())
            .request(AIRTABLE_API_KEY)
            .await
            .unwrap();

    match records.first() {
        Some(submission) => return HttpResponse::Ok().json(submission),
        None => {
            return HttpResponse::NotFound().json(json!(
                r#"{"status": 404, "message": "No additional submissions to review."}"#
            ))
        }
    }
}

#[get("/test")]
async fn test(base: web::Data<Base>) -> impl Responder {
    let data = base.query().await.unwrap();
    let file = File::create("records.json").unwrap();
    serde_json::to_writer_pretty(file, &data).unwrap();

    HttpResponse::Ok()
        .content_type("application/json")
        .body(r#"{"status": 200, "message": "wrote data to disk"}"#)
}

#[post("/update")]
async fn update(submission: web::Json<Record<Submission>>) -> impl Responder {
    // base.update_records(&[submission.into_inner()]).await.unwrap();

    HttpResponse::Ok()
        .content_type("application/json")
        .body(r#"{"status": 200, "message": "updated records"}"#)
}

#[get("/icon-uri")]
async fn icon_uri() -> impl Responder {
    let icon = base64::prelude::BASE64_STANDARD.encode(ICON);
    let uri = IMAGE_DATA_URI.to_owned() + &icon;

    HttpResponse::Ok()
        .content_type("text/plain")
        .body(uri)
}

#[derive(Deserialize)]
struct ReviewData {
    id: RecordId,
    status: String,
}

#[post("/review")]
async fn review(submission: web::Json<ReviewData>) -> impl Responder {
    println!("hi!");

    let rec: Record<Submission> = airtable::api::get_record(AIRTABLE_API_KEY, AIRTABLE_BASE_ID, SUBMISSION_TABLE, &submission.id).await.unwrap();
    let mut data = rec.into_fields();
    data.status = submission.status.clone();

    airtable::api::update_record(AIRTABLE_API_KEY, AIRTABLE_BASE_ID, SUBMISSION_TABLE, &submission.id, data, false).await.unwrap();

    HttpResponse::Ok()
        .content_type("application/json")
        .body(r#"{"status": 200, "message": "updated submission"}"#)
}

#[get("/updatetest")]
async fn update_test() -> impl Responder {
    let records: Vec<Record<Submission>> =
        ListRecords::new(AIRTABLE_BASE_ID.to_owned(), SUBMISSION_TABLE.to_owned())
            .with_view(TABLE_VIEW.to_owned())
            .request(AIRTABLE_API_KEY)
            .await
            .unwrap();

    let mut test_record = records[68].fields().clone();
    test_record.status = "accepted".to_owned();

    airtable::api::update_record(
        AIRTABLE_API_KEY,
        AIRTABLE_BASE_ID,
        SUBMISSION_TABLE,
        records[68].id(),
        test_record,
        false,
    )
    .await
    .unwrap();

    HttpResponse::Ok()
        .content_type("application/json")
        .body(r#"{"status": 200,"message":"updated record"}"#)
}

#[get("/")]
async fn index() -> impl Responder {
    NamedFile::open_async("./static/index.html").await
}

#[get("/favicon.ico")]
async fn favicon() -> impl Responder {
    NamedFile::open_async("./static/index.html").await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    std::env::set_var("RUST_LOG", "actix_web=trace");
    env_logger::init();

    let base = Base::new(
        AIRTABLE_API_KEY.to_owned(),
        AIRTABLE_BASE_ID,
        SUBMISSION_TABLE,
        TABLE_VIEW,
    );

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(base.clone()))
            .service(record)
            .service(next_record)
            .service(index)
            .service(favicon)
            .service(test)
            .service(update)
            .service(update_test)
            .service(icon_uri)
            .service(review)
            .service(Files::new("/static", "static").prefer_utf8(true))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
