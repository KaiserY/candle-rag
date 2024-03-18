CREATE TABLE file (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  filename TEXT NOT NULL,
  bytes INTEGER NOT NULL,
  purpose TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE UNIQUE INDEX idx_unique_filename_purpose ON file (filename, purpose);

CREATE TABLE embedding (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE UNIQUE INDEX idx_unique_name ON embedding (name);

CREATE TABLE embedding_file (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  embedding_id INTEGER NOT NULL,
  file_id INTEGER NOT NULL,
);

CREATE UNIQUE INDEX idx_unique_embedding_id_file_id ON embedding_file (embedding_id, file_id);

CREATE TABLE embedding_vector (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  embedding_id INTEGER NOT NULL,
  vector_id INTEGER NOT NULL,
);

CREATE UNIQUE INDEX idx_unique_embedding_id_vector_id ON embedding_file (embedding_id, vector_id);
