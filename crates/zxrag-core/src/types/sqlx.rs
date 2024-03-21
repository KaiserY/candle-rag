use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, sqlx::FromRow)]
pub struct File {
  pub id: i64,
  pub kb_id: i64,
  pub filename: String,
  pub bytes: i64,
  pub purpose: String,
  pub created_at: i64,
  pub updated_at: i64,
}

#[derive(Debug, Default, Serialize, Deserialize, sqlx::FromRow)]
pub struct KnowledgeBase {
  pub id: i64,
  pub name: String,
  pub created_at: i64,
  pub updated_at: i64,
}
