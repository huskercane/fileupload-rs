// Instead of #[macro_use], newer versions of rust should prefer
use strum_macros::EnumString;

// Encapsulates useful properties which are
// provided by Rocket on start (see main function).
#[derive(Deserialize)]
pub struct ConfigurationMain {
    pub(crate) development: Configuration,
    pub(crate) staging: Configuration,
    pub(crate) production: Configuration,
}

#[derive(Deserialize, Clone)]
pub struct Configuration {
    pub(crate) retention_time: u64,
    pub(crate) download_url: String,
    pub(crate) file_storage_location: String,
}

#[derive(Deserialize, PartialEq, EnumString, Clone)]
pub(crate) enum Environment {
    #[strum(serialize = "development", serialize = "d")]
    DEVELOPMENT,
    #[strum(serialize = "staging", serialize = "s")]
    STAGING,
    #[strum(serialize = "production", serialize = "p")]
    PRODUCTION,
}