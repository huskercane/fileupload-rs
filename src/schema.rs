table! {
    file_storage (id) {
        id -> Integer,
        file_name -> Text,
        storage_name -> Text,
        created_at -> Timestamp,
        modified_at -> Timestamp,
        is_deleted -> Bool,
    }
}
