-- Your SQL goes here
CREATE TABLE file_storage
(
    id           INTEGER PRIMARY KEY,
    file_name    VARCHAR NOT NULL,
    storage_name VARCHAR NOT NULL,
    created_at   DATETIME         DEFAULT CURRENT_TIMESTAMP,
    modified_at  DATETIME         DEFAULT CURRENT_TIMESTAMP,
    is_deleted   BOOLEAN NOT NULL DEFAULT 'f'
)