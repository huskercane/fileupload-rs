extern crate actix_web;
extern crate chrono;
#[macro_use]
extern crate diesel;
extern crate dotenv;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate strum_macros;

use std::{env, fs};
use std::io::Write;

use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::{App, Error, get, HttpServer, post, web};
use actix_web::error::ErrorPreconditionFailed;
use actix_web::rt::spawn;
use chrono::{Duration, Local, TimeZone, Utc};
use diesel::prelude::*;
use dotenv::dotenv;
use futures::{StreamExt, TryStreamExt};
use futures::prelude::*;
use futures_timer::Delay;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use rand::distributions::Alphanumeric;
use rand::Rng;

use schema::file_storage;

use crate::models::{FileStorage, NewFileStorage};
use crate::models::configuration::Configuration;

mod models;
mod schema;
mod config;

// TODO: look at pool in document
#[derive(Debug, Serialize)]
pub struct FileDetail {
    pub download_url: String,
    pub expiry: String,
}

#[post("/")]
async fn save_file(mut payload: Multipart, data: web::Data<Configuration>) -> Result<String, Error> {
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

        let filepath = format!("{}/{}-{}", data.file_storage_location, random_name, incoming_file_name);

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
    let response = FileDetail {
        download_url: format!("https://{}/{}", data.download_url, random_name),
        expiry: one_hour_from_now.unwrap().format("%Y-%m-%d %H:%M:%S").to_string(),
    };
    let storage_record = create_file_storage(&incoming_file_name, &random_name).unwrap();

    let duration = std::time::Duration::from_secs(data.retention_time);
    let now_future = Delay::new(duration);
    spawn(now_future.map(move |()| {
        let filepath = format!("{}/{}-{}", data.file_storage_location, random_name, incoming_file_name);

        info!("Delete file now - {}", filepath);
        // delete file
        let b = std::path::Path::new(&filepath).exists();
        if b {
            fs::remove_file(&filepath).unwrap();
        }
        // update db
        let result = update_deleted(storage_record.id);
        info!("Done Delete file now {}-{}", result.unwrap().to_string(), filepath);
    }));

    return Ok(serde_json::to_string(&response).unwrap());
}

fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}",
                                   database_url))
}

pub fn create_file_storage(file_name: &str, storage_name: &str) -> Option<FileStorage> {
    let connection = establish_connection();

    let new_post = NewFileStorage { file_name, storage_name };

    let result = diesel::insert_into(file_storage::table)
        .values(&new_post)
        .execute(&connection)
        .expect("Error saving new post");
    debug!("Rows inserted {}", result.to_string());

    get_file_name(storage_name)
}

pub fn get_file_name(download_name: &str) -> Option<FileStorage> {
    let connection = establish_connection();
    schema::file_storage::dsl::file_storage
        .filter(schema::file_storage::storage_name.eq(download_name))
        .first(&connection)
        .optional()
        .unwrap()
}

pub fn get_all_file_name() -> Vec<FileStorage> {
    let connection = establish_connection();
    return schema::file_storage::dsl::file_storage
        .filter(schema::file_storage::is_deleted.eq(false))
        .load::<FileStorage>(&connection)
        .expect("Error loading FileStorage");
}

pub fn update_deleted(id: i32) -> QueryResult<usize> {
    let connection = establish_connection();

    let target = schema::file_storage::dsl::file_storage
        .filter(schema::file_storage::id.eq(id));
    let result = diesel::update(target)
        .set(schema::file_storage::is_deleted.eq(true))
        .execute(&connection);
    result
}

#[get("/files/{file_name}")]
async fn download_file(web::Path(file_name_x): web::Path<String>, data: web::Data<Configuration>) -> Result<NamedFile, Error> {
    let sanitize_file_name = sanitize_filename::sanitize(&file_name_x);
    let real_file_name = get_file_name(&sanitize_file_name);
    match real_file_name {
        None => {
            Err(ErrorPreconditionFailed(format!("Unable to find file: {}", sanitize_file_name)))
        }
        Some(ff_name) => {
            let filepath = format!("{}/{}-{}", data.file_storage_location, ff_name.storage_name, ff_name.file_name);
            Ok(NamedFile::open(filepath)?)
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // env_logger::Builder::from_env("trace").init();
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info,main=debug");
    env_logger::init();

    let app_configuration = config::load_config();

    std::fs::create_dir_all(app_configuration.clone().file_storage_location).unwrap();

    let now = Utc::now();

    let all_not_deleted_files = get_all_file_name();
    for not_deleted_file in all_not_deleted_files {
        let naive = not_deleted_file.created_at;
        let file_path = format!("{}/{}-{}", app_configuration.clone().file_storage_location, not_deleted_file.storage_name, not_deleted_file.file_name);
        let u = Local.from_local_datetime(&naive).unwrap();
        let y = now.signed_duration_since(u);
        if y.num_seconds() > app_configuration.retention_time as i64 {
            let b = std::path::Path::new(&file_path).exists();
            if b {
                fs::remove_file(&file_path).unwrap();
            }
            // update db
            let result = update_deleted(not_deleted_file.id);
            debug!("Update size {}", result.unwrap().to_string());
        } else if y > 0 {
            // add a task to delete
            let duration = std::time::Duration::from_secs(y.num_seconds() as u64);
            let now_future = Delay::new(duration);
            let x = now_future.map(move |()| {
                // let string = format!("Delete file now - {}", file_name);
                info!("Delete file now - {}", file_path);
                // delete file
                let b = std::path::Path::new(&file_path).exists();
                if b {
                    fs::remove_file(&file_path).unwrap();
                }
                // update db
                let result = update_deleted(not_deleted_file.id);
                info!("Done cleaning up {}", result.unwrap().to_string());
            });
            spawn(x);
        }
    }

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

    let y = app_configuration.clone();
    let client = web::Data::new(app_configuration);

    HttpServer::new(move || {
        App::new()
            .app_data(client.clone())
            .service(save_file)
            .service(download_file)
    })
        .bind_openssl(y.download_url, builder)?
        .run()
        .await
}