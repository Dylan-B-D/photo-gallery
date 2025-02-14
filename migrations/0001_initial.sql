-- migrations/0001_initial.sql
CREATE TABLE albums (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    date TEXT NOT NULL,
    num_images INTEGER DEFAULT 0,
    camera_model TEXT,
    lens_model TEXT,
    aperture TEXT
);

CREATE TABLE images (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    album_id INTEGER NOT NULL,
    filename TEXT NOT NULL,
    camera_make TEXT,
    camera_model TEXT,
    lens_model TEXT,
    iso TEXT,
    aperture TEXT,
    shutter_speed TEXT,
    focal_length TEXT,
    light_source TEXT,
    date_created TEXT,
    file_size INTEGER,
    FOREIGN KEY (album_id) REFERENCES albums (id)
);