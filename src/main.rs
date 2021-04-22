extern crate actix_web;
extern crate chrono;
#[macro_use]
extern crate diesel;
extern crate dotenv;
extern crate env_logger;
extern crate log;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use std::env;
use std::io::Write;

use actix_files::{NamedFile};
use actix_multipart::Multipart;
use actix_web::{App, Error, get, HttpServer, post, web};
use actix_web::error::{ErrorPreconditionFailed};
use chrono::{Duration, Utc};
use diesel::prelude::*;
use dotenv::dotenv;
use futures::{StreamExt, TryStreamExt};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use rand::distributions::Alphanumeric;
use rand::Rng;

use schema::file_storage;

use crate::models::{NewFileStorage, FileStorage};

mod models;
mod schema;

// TODO: tasks - https://www.reddit.com/r/rust/comments/fddf6y/handling_longrunning_background_tasks_in_actixweb/
// TODO: on startup loop over all files in db
// TODO: delete all expired
// TODO: update database to set expired and updated at time stamp
// TODO: set timer for rest
// TODO: look at pool in document
#[derive(Debug, Serialize)]
pub struct FileDetail {
    pub download_url: String,
    pub expiry: String,
}

#[post("/")]
async fn save_file(mut payload: Multipart) -> Result<String, Error> {
    // iterate over multipart stream
    let random_name: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect();

    let now = Utc::now();
    let one_hour_from_now = now.checked_add_signed(Duration::hours(1));
    let mut incoming_file_name = "".to_string(); // TODO: fix me

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_type = field.content_disposition().unwrap();
        let filename = content_type.get_filename().unwrap();

        incoming_file_name = sanitize_filename::sanitize(&filename);

        let filepath = format!("./tmp/{}-{}", random_name, incoming_file_name);

        // File::create is blocking operation, use threadpool
        let mut f = web::block(|| std::fs::File::create(filepath))
            .await
            .unwrap();

        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            // filesystem operations are blocking, we have to use threadpool
            f = web::block(move || f.write_all(&data).map(|_| f)).await?;
        }
    }
    let x = FileDetail {
        download_url: format!("https://127.0.0.1:8080/{}", random_name),
        expiry: one_hour_from_now.unwrap().format("%Y-%m-%d %H:%M:%S").to_string(),
    };
    create_file_storage(&incoming_file_name, &random_name);
    return Ok(serde_json::to_string(&x).unwrap());
}

fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}",
                                   database_url))
}

pub fn create_file_storage(file_name: &str, storage_name: &str) {
    let connection = establish_connection();

    let new_post = NewFileStorage { file_name, storage_name };

    diesel::insert_into(file_storage::table)
        .values(&new_post)
        .execute(&connection)
        .expect("Error saving new post");
}

pub fn get_file_name(download_name: &str) -> Option<FileStorage> {
    let connection = establish_connection();
    schema::file_storage::dsl::file_storage
        .filter(schema::file_storage::storage_name.eq(download_name))
        .first(&connection)
        .optional()
        .unwrap()
}

#[get("/files/{file_name}")]
async fn download_file(web::Path(file_name_x): web::Path<String>) -> Result<NamedFile, Error> {
    let sanitize_file_name = sanitize_filename::sanitize(&file_name_x);
    let real_file_name = get_file_name(&sanitize_file_name);
    match real_file_name {
        None => {
            Err(ErrorPreconditionFailed(format!("Unable to find file: {}", sanitize_file_name)))
        }
        Some(ff_name) => {
            let filepath = format!("./tmp/{}-{}", ff_name.storage_name,ff_name.file_name);
            Ok(NamedFile::open(filepath)?)
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // env_logger::Builder::from_env("trace").init();
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    env_logger::init();
    std::fs::create_dir_all("./tmp").unwrap();

    // Find which env this is if nothing then development
    // update object
    // load ssl keys
    // to create a self-signed temporary cert for testing:
    // `openssl req -x509 -newkey rsa:4096 -nodes -keyout key.pem -out cert.pem -days 365 -subj '/CN=localhost'`
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file("config/key.pem", SslFiletype::PEM)
        .unwrap();
    builder
        .set_certificate_chain_file("config/cert.pem")
        .unwrap();


    HttpServer::new(|| {
        App::new()
            .service(save_file)
            .service(download_file)
    })
        .bind_openssl("127.0.0.1:8080", builder)?
        .run()
        .await
}