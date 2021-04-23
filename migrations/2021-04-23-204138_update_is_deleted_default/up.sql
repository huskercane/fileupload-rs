-- Your SQL goes here
CREATE TABLE file_storage_new
(
    id           INTEGER  NOT NULL PRIMARY KEY,
    file_name    VARCHAR  NOT NULL,
    storage_name VARCHAR  NOT NULL,
    created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    modified_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_deleted   BOOLEAN  NOT NULL DEFAULT 0
);
-- copy data from the table to the new_table
INSERT INTO file_storage_new(id, file_name, storage_name, created_at, modified_at, is_deleted)
SELECT id, file_name, storage_name, created_at, modified_at, is_deleted
FROM file_storage;

-- drop the table
DROP TABLE file_storage;

-- rename the new_table to the table
ALTER TABLE file_storage_new
    RENAME TO file_storage;

CREATE INDEX IF NOT EXISTS file_storage_is_deleted ON file_storage (is_deleted);
CREATE UNIQUE INDEX IF NOT EXISTS file_storage_storage_name ON file_storage (storage_name)