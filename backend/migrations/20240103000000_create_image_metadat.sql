CREATE TABLE IF NOT EXISTS image_metadata (
    image_id TEXT PRIMARY KEY REFERENCES images(id),
    camera_make TEXT,
    camera_model TEXT,
    lens_model TEXT,
    iso INTEGER,
    aperture REAL,
    shutter_speed TEXT,
    focal_length REAL
);
