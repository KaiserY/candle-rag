CREATE TABLE IF NOT EXISTS file (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  filename TEXT NOT NULL,
  bytes INTEGER NOT NULL,
  purpose TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_unique_filename_purpose ON file (filename, purpose);

CREATE TABLE IF NOT EXISTS knowledge_base (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_unique_name ON knowledge_base (name);

CREATE TABLE IF NOT EXISTS knowledge_base_file (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  kb_id INTEGER NOT NULL,
  file_id INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_unique_kb_id_file_id ON knowledge_base_file (kb_id, file_id);

CREATE TABLE IF NOT EXISTS knowledge_base_vector (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  kb_id INTEGER NOT NULL,
  vector_id INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_unique_kb_id_vector_id ON knowledge_base_vector (kb_id, vector_id);
