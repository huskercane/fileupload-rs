use std::fs;
use std::io::Write;

use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::error::ErrorPreconditionFailed;
use actix_web::rt::spawn;
use actix_web::{get, post, web, Error, HttpRequest, HttpResponse};
use chrono::{Duration, Utc};
use futures::prelude::*;
use futures::{StreamExt, TryStreamExt};
use futures_timer::Delay;
use rand::distributions::Alphanumeric;
use rand::Rng;

use crate::db::{create_file_storage, get_file_name, update_deleted, DbPool};
use crate::encrypt_file::{EncryptFile, UploadedFile};
use crate::models::configuration::Configuration;
use crate::models::upload_file::FileDetail;
use actix_web::http::HeaderValue;

#[post("/")]
pub(crate) async fn save_file(
    request: HttpRequest,
    mut payload: Multipart,

    app_configuration: web::Data<Configuration>,
    pool: web::Data<DbPool>,
) -> Result<String, Error> {
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

        let filepath = format!(
            "{}/{}-{}",
            app_configuration.file_storage_location, random_name, incoming_file_name
        );

        // File::create is blocking operation, use threadpool
        let mut f = web::block(|| std::fs::File::create(filepath))
            .await
            .unwrap();

        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.next().await {
            let data_in = chunk.unwrap();
            // filesystem operations are blocking, we have to use threadpool
            f = web::block(move || f.write_all(&data_in).map(|_| f)).await?;
        }
    }
    let random_name_2 = random_name.clone();
    let incoming_file_name_2 = incoming_file_name.clone();
    let x1 = request.headers().clone().to_owned();
    info!("XXX {}", x1.contains_key("PASSWORD"));
    let file_password = x1.get("PASSWORD");

    let string = app_configuration.file_storage_location.clone();
    match file_password {
        None => {
            info!("No password so not encrypting")
        }

        Some(password) => {
            // let file_encrypt_future = async move {
            encrypt_file(string, random_name_2, incoming_file_name_2, password).await;
        }
    };

    let response = FileDetail {
        download_url: format!("https://{}/{}", app_configuration.download_url, random_name),
        expiry: one_hour_from_now
            .unwrap()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
    };

    // clone them because we need to pass them to two closures
    let ifn = incoming_file_name.clone();
    let rn = random_name.clone();
    let p2 = pool.clone();

    let storage_record =
        web::block(move || create_file_storage(&incoming_file_name, &random_name, &pool))
            .await
            .map_err(|e| {
                eprintln!("{}", e);
                HttpResponse::InternalServerError().finish()
            })?;

    let duration = std::time::Duration::from_secs(app_configuration.retention_time);
    let now_future = Delay::new(duration);

    spawn(now_future.map(move |()| {
        let filepath = format!("{}/{}-{}", app_configuration.file_storage_location, rn, ifn);

        info!("Delete file now - {}", filepath);
        // delete file
        let b = std::path::Path::new(&filepath).exists();
        if b {
            fs::remove_file(&filepath).unwrap();
        }
        // update db
        let result = update_deleted(storage_record.unwrap().id, &p2);
        info!(
            "Done Delete file now {}-{}",
            result.unwrap().to_string(),
            filepath
        );
    }));

    return Ok(serde_json::to_string(&response).unwrap());
}

async fn encrypt_file(
    file_save_location: String,
    random_name_2: String,
    incoming_file_name_2: String,
    val: &HeaderValue,
) {
    let uploaded_file_name = format!(
        "{}/{}-{}",
        file_save_location,
        random_name_2,
        incoming_file_name_2.clone()
    );
    let x = UploadedFile {
        file_name: uploaded_file_name.clone(),
        saved_file: format!("{}.enc", uploaded_file_name.clone()),
        password: Some(String::from(val.to_str().unwrap())),
        private_key: None,
    };

    x.encrypt();
}

#[get("/files/{file_name}")]
pub(crate) async fn download_file(
    web::Path(file_name_x): web::Path<String>,
    data: web::Data<Configuration>,
    pool: web::Data<DbPool>,
) -> Result<NamedFile, Error> {
    let sanitize_file_name = sanitize_filename::sanitize(&file_name_x);
    let real_file_name = get_file_name(&sanitize_file_name, &pool).unwrap();
    match real_file_name {
        None => Err(ErrorPreconditionFailed(format!(
            "Unable to find file: {}",
            sanitize_file_name
        ))),
        Some(ff_name) => {
            let filepath = format!(
                "{}/{}-{}",
                data.file_storage_location, ff_name.storage_name, ff_name.file_name
            );
            info!("Delete file after download - {}", filepath);
            // delete file
            let b = std::path::Path::new(&filepath).exists();
            if b {
                fs::remove_file(&filepath).unwrap();
            }

            web::block(move || update_deleted(ff_name.id, &pool))
                .await
                .map_err(|e| {
                    error!("{}", e);
                    HttpResponse::InternalServerError().finish()
                })?;
            Ok(NamedFile::open(filepath)?)
        }
    }
}
