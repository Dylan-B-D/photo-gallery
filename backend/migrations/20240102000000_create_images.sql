CREATE TABLE IF NOT EXISTS images (
    id TEXT PRIMARY KEY,
    album_id TEXT NOT NULL,
    file_name TEXT NOT NULL,
    FOREIGN KEY (album_id) REFERENCES albums(id)
);