// fn establish_connection() -> SqliteConnection {
//     dotenv().ok();
//
//     let database_url = env::var("DATABASE_URL")
//         .expect("DATABASE_URL must be set");
//     SqliteConnection::establish(&database_url)
//         .unwrap_or_else(|_| panic!("Error connecting to {}",
//                                    database_url))
// }
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;

use schema::file_storage;

use crate::models::{FileStorage, NewFileStorage};
use crate::schema;

pub(crate) type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

pub fn create_file_storage(file_name: &str, storage_name: &str, pool: &DbPool) -> Result<Option<FileStorage>, diesel::result::Error> {
    let connection = pool.get().expect("couldn't get db connection from pool");

    let new_post = NewFileStorage { file_name, storage_name };

    let result = diesel::insert_into(file_storage::table)
        .values(&new_post)
        .execute(&connection)
        .expect("Error saving new post");
    debug!("Rows inserted {}", result.to_string());

    get_file_name(storage_name, pool)
}

pub fn get_file_name(download_name: &str, pool: &DbPool) -> Result<Option<FileStorage>, diesel::result::Error> {
    let connection = pool.get().expect("couldn't get db connection from pool");
    schema::file_storage::dsl::file_storage
        .filter(schema::file_storage::storage_name.eq(download_name))
        .first(&connection)
        .optional()
}

pub fn get_all_file_name(pool: &DbPool) -> Result<Vec<FileStorage>, diesel::result::Error> {
    let connection = pool.get().expect("couldn't get db connection from pool");
    let result = schema::file_storage::dsl::file_storage
        .filter(schema::file_storage::is_deleted.eq(false))
        .load::<FileStorage>(&connection)
        .expect("Error loading FileStorage");

    return Ok(result);
}

pub fn update_deleted(id: i32, pool: &DbPool) -> Result<usize, diesel::result::Error> {
    let connection = pool.get().expect("couldn't get db connection from pool");

    let target = schema::file_storage::dsl::file_storage
        .filter(schema::file_storage::id.eq(id));
    let result = diesel::update(target)
        .set(schema::file_storage::is_deleted.eq(true))
        .execute(&connection);
    result
}
