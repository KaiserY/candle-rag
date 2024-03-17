CREATE TABLE file (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  filename TEXT NOT NULL,
  bytes INTEGER NOT NULL,
  purpose TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE UNIQUE INDEX idx_unique_filename_purpose ON file (filename, purpose);
