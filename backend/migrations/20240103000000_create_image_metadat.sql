CREATE TABLE IF NOT EXISTS image_metadata (
    image_id TEXT PRIMARY KEY,
    
    -- Camera Info
    camera_make TEXT,
    camera_model TEXT,
    lens_model TEXT,
    
    -- Technical Details
    iso INTEGER,
    aperture REAL,
    shutter_speed TEXT,
    focal_length REAL,
    
    -- Image Details
    light_source TEXT,
    
    -- Time and Location
    date_created TEXT,
    
    -- File Info
    file_size INTEGER,
    
    FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE CASCADE
);