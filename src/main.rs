use std::fs::File;

use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use airtable::{Attachment, Base, Record};
use serde::{Deserialize, Serialize};
use serde_json::json;

mod airtable;

const AIRTABLE_API_KEY: &str = env!("AIRTABLE_API_KEY");
const AIRTABLE_BASE_ID: &str = env!("AIRTABLE_BASE_ID");
const INDEX: &str = include_str!("index.html");
const EMAIL: &str = include_str!("email.html");
const ICON: &[u8; 76109] = include_bytes!("say-cheese.png");

const SUBMISSION_TABLE: &str = "YSWS Project Submission";
const TABLE_VIEW: &str = "Grid View";

// listen i didnt name the records dont blame me
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Submission {
    #[serde(rename = "Code URL")]
    repo_url: String,
    #[serde(rename = "Screenshot")]
    screenshot: Vec<Attachment>,
    #[serde(rename = "Description")]
    description: String,
    #[serde(rename = "Optional - Override Hours Spent")]
    hours: f32,
    #[serde(rename = "qr_code")]
    qr_code: Vec<Attachment>,
    #[serde(rename = "gallery_attribution")]
    gallery_attribution: String,
    #[serde(rename = "os")]
    os: String,
    #[serde(rename = "architecture")]
    architecture: String,
    #[serde(rename = "project_name")]
    name: String,
    #[serde(rename = "Automation - Status")]
    status: String,
}

#[get("/record/{i}")]
async fn record(base: web::Data<Base>, i: web::Path<usize>) -> impl Responder {
    let json: Vec<Record<Submission>> = base.read_records().await.expect("should be able to query base");
    json.get(i.into_inner()).cloned().map(web::Json)
}

#[get("/nextrecord")]
async fn next_record(base: web::Data<Base>) -> impl Responder {
    let json: Vec<Record<Submission>> = base.read_records().await.expect("should be able to access records");
    for submission in json {
        if submission.fields().status == "1-Pending Submission" {
            continue;
        }

        return HttpResponse::Ok().json(submission);
    }

    return HttpResponse::NotFound().json(json!(r#"{"status": 404, "message": "No additional submissions to review."}"#))
}

#[get("/test")]
async fn test(base: web::Data<Base>) -> impl Responder {
    let data = base.query().await.unwrap();
    let file = File::create("records.json").unwrap();
    serde_json::to_writer_pretty(file, &data).unwrap();

    HttpResponse::Ok().content_type("text/json").body(r#"{"status": 200, "message": "wrote data to disk"}"#)
}

#[post("/update")]
async fn update(base: web::Data<Base>, submission: web::Json<Record<Submission>>) -> impl Responder {
    base.update_records(&[submission.into_inner()]).await.unwrap();

    HttpResponse::Ok().content_type("text/json").body(r#"{"status": 200, "message": "updated records"}"#)
}

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().content_type("text/html").body(INDEX)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let base = Base::new(
        AIRTABLE_API_KEY.to_owned(),
        AIRTABLE_BASE_ID,
        SUBMISSION_TABLE,
        TABLE_VIEW,
    );

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(base.clone()))
            .service(record)
            .service(next_record)
            .service(index)
            .service(test)
            .service(update)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
