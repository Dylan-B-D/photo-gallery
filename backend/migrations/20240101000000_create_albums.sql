CREATE TABLE IF NOT EXISTS albums (
    id TEXT PRIMARY KEY,              
    name TEXT NOT NULL,
    description TEXT,
    date TEXT,
    number_of_images INTEGER NOT NULL DEFAULT 0
);
