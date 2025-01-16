use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use airtable::{Attachment, Base, Record};
use serde::{Deserialize, Serialize};
use serde_json::json;

mod airtable;

const AIRTABLE_API_KEY: &str = env!("AIRTABLE_API_KEY");
const AIRTABLE_BASE_ID: &str = env!("AIRTABLE_BASE_ID");
const INDEX: &str = include_str!("index.html");

const SUBMISSION_TABLE: &str = "YSWS Project Submission";
const TABLE_VIEW: &str = "Grid View";

// listen i didnt name the records dont blame me
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Submission {
    #[serde(rename(deserialize = "Code URL"))]
    repo_url: String,
    #[serde(rename(deserialize = "Screenshot"))]
    screenshot: Vec<Attachment>,
    #[serde(rename(deserialize = "Description"))]
    description: String,
    #[serde(rename(deserialize = "Optional - Override Hours Spent"))]
    hours: f32,
    #[serde(rename(deserialize = "qr_code"))]
    qr_code: Vec<Attachment>,
    #[serde(rename(deserialize = "gallery_attribution"))]
    gallery_attribution: String,
    #[serde(rename(deserialize = "os"))]
    os: String,
    #[serde(rename(deserialize = "architecture"))]
    architecture: String,
    #[serde(rename(deserialize = "project_name"))]
    name: String,
    #[serde(rename(deserialize = "Automation - Status"))]
    status: String,
}

#[get("/record/{i}")]
async fn record(base: web::Data<Base>, i: web::Path<usize>) -> impl Responder {
    let json: Vec<Record<Submission>> = base.records().await.expect("should be able to query base");
    json.get(i.into_inner()).cloned().map(web::Json)
}

#[get("/nextrecord")]
async fn next_record(base: web::Data<Base>) -> impl Responder {
    let json: Vec<Record<Submission>> = base.records().await.expect("should be able to access records");
    for submission in json {
        if submission.fields().status == "1-Pending Submission" {
            continue;
        }

        return HttpResponse::Ok().json(submission);
    }

    return HttpResponse::NotFound().json(json!(r#"{"status": 404, "message": "No additional submissions to review."}"#))
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
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
