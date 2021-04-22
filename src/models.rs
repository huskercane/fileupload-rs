use crate::file_storage;

#[derive(Serialize, Queryable)]
pub struct FileStorage {
    pub id: i32,
    pub file_name: String,
    pub storage_name: String,
    pub created_at: chrono::NaiveDateTime,
    pub modified_at: chrono::NaiveDateTime,
    pub is_deleted: bool,
}

#[derive(Insertable)]
#[table_name = "file_storage"]
pub struct NewFileStorage<'a> {
    pub file_name: &'a str,
    pub storage_name: &'a str,
}