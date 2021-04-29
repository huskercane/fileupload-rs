extern crate actix_web;
extern crate chrono;
#[macro_use]
extern crate diesel;
extern crate dotenv;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate r2d2;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate strum_macros;

use std::{env, fs};

use actix_web::{App, HttpServer, middleware, web};
use actix_web::rt::spawn;
use chrono::{ TimeZone, Utc};
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use dotenv::dotenv;
use futures::prelude::*;
use futures_timer::Delay;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

use schema::file_storage;

use crate::db::*;
use crate::web_function::*;

mod models;
mod schema;
mod config;
mod db;
mod web_function;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let app_configuration = config::load_config();

    std::fs::create_dir_all(app_configuration.clone().file_storage_location).unwrap();

    let now = Utc::now();

    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    // set up database connection pool
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    let builder = r2d2::Pool::builder();
    let pool = builder
        .build(manager)
        .expect("Failed to create pool.");

    let all_not_deleted_files = get_all_file_name(&pool).unwrap();
    for not_deleted_file in all_not_deleted_files {
        let created_at = not_deleted_file.created_at;
        let utc_time = Utc.from_local_datetime(&created_at).unwrap();
        let duration_since_create = now.signed_duration_since(utc_time);

        let file_path = format!("{}/{}-{}", app_configuration.clone().file_storage_location, not_deleted_file.storage_name, not_deleted_file.file_name);

        let duration_in_seconds = duration_since_create.num_seconds();
        if duration_in_seconds > app_configuration.retention_time as i64 {
            let b = std::path::Path::new(&file_path).exists();
            if b {
                fs::remove_file(&file_path).unwrap();
            }
            // update db
            let result = update_deleted(not_deleted_file.id, &pool);
            debug!("Update size {}", result.unwrap().to_string());
        } else if duration_in_seconds > 0 {
            // add a task to delete
            let duration = std::time::Duration::from_secs(duration_in_seconds as u64);
            let now_future = Delay::new(duration);
            let x = now_future.map(move |()| {
                let database_url = env::var("DATABASE_URL")
                    .expect("DATABASE_URL must be set");
                let builder_1 = r2d2::Pool::builder();
                let manager_2 = ConnectionManager::<SqliteConnection>::new(database_url);
                let pool_2 = builder_1
                    .build(manager_2)
                    .expect("Failed to create pool.");
                info!("Delete file now - {}", file_path);
                // delete file
                let b = std::path::Path::new(&file_path).exists();
                if b {
                    fs::remove_file(&file_path).unwrap();
                }
                // update db
                let result = update_deleted(not_deleted_file.id, &pool_2);
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
    let configuration = web::Data::new(app_configuration);

    info!("Using config: {}", config::load_config());

    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .app_data(configuration.clone())
            .wrap(middleware::Logger::default())
            .service(save_file)
            .service(download_file)
    })
        .bind_openssl(y.download_url, builder)?
        .run()
        .await
}