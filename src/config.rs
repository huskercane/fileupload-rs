extern crate cached;
extern crate toml;

use std::env;
use std::fs::File;
use std::io::Read;

use cached::proc_macro::cached;

use crate::models::configuration::{Configuration, ConfigurationMain};

#[cached]
pub fn load_config() -> Configuration {
    let mut config_toml = String::new();

    let path = "./config.toml";
    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(_) => {
            return Configuration {
                retention_time: 0,
                download_url: String::from(""),
                file_storage_location: String::from(""),
            };
        }
    };

    file.read_to_string(&mut config_toml)
        .unwrap_or_else(|err| panic!("Error while reading config: [{}]", err));

    let env = env::var("ENVIRONMENT").unwrap_or("development".to_string());

    let all_config: ConfigurationMain = toml::from_str(&config_toml).unwrap();
    let mut config = all_config.development;
    if env == "STAGING" {
        config = all_config.staging;
    }
    if env == "PRODUCTION" {
        config = all_config.production;
    }

    let retention_time = config.retention_time;
    let download_url = String::from(config.download_url);
    let file_storage_location = String::from(config.file_storage_location);
    return Configuration {
        retention_time,
        download_url,
        file_storage_location,
    };
}
