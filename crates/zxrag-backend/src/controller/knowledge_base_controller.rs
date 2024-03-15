use arrow_array::RecordBatchIterator;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Json};
use serde::{Deserialize, Serialize};
use zxrag_core::types::lancedb::get_embedding_schema;

use crate::error::BackendError;
use crate::types::openai::ChatCompletionRequest;
use crate::BackendState;

pub async fn create_tables(
  State(state): State<BackendState>,
  Json(req): Json<CreateTableRequest>,
) -> Result<impl IntoResponse, BackendError> {
  let db = vectordb::connect(&state.config.lancedb_path)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  let tables = db.table_names().await.map_err(|e| anyhow::anyhow!(e))?;

  if tables.contains(&req.name) {
    Ok(Json(CreateTableResponse { name: req.name }))
  } else {
    let schema = get_embedding_schema()?;

    let batches = RecordBatchIterator::new(vec![], schema);

    let tbl = db
      .create_table(&req.name, Box::new(batches), None)
      .await
      .map_err(|e| anyhow::anyhow!(e))?;

    tracing::info!("{}", tbl);

    Ok(Json(CreateTableResponse { name: req.name }))
  }
}

pub async fn list_tables(
  State(state): State<BackendState>,
) -> Result<impl IntoResponse, BackendError> {
  let db = vectordb::connect(&state.config.lancedb_path)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  let tables = db.table_names().await.map_err(|e| anyhow::anyhow!(e))?;

  let data = tables
    .into_iter()
    .map(|t| TableDescription { name: t })
    .collect();

  Ok(Json(ListTableResponse { data }))
}

pub async fn delete_table(
  State(state): State<BackendState>,
  Json(req): Json<DeleteTableRequest>,
) -> Result<impl IntoResponse, BackendError> {
  let db = vectordb::connect(&state.config.lancedb_path)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  let tables = db.table_names().await.map_err(|e| anyhow::anyhow!(e))?;

  if tables.contains(&req.name) {
    db.drop_table(&req.name)
      .await
      .map_err(|e| anyhow::anyhow!(e))?;
  }

  Ok(Json(DeleteTableResponse { name: req.name }))
}

pub async fn create_chat_completion(
  State(_state): State<BackendState>,
  Path(_table_id): Path<String>,
  Json(_req): Json<ChatCompletionRequest<'_>>,
) -> Result<impl IntoResponse, BackendError> {
  Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct CreateTableRequest {
  name: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateTableResponse {
  name: String,
}

#[derive(Serialize, Deserialize)]
pub struct ListTableResponse {
  data: Vec<TableDescription>,
}

#[derive(Serialize, Deserialize)]
pub struct TableDescription {
  name: String,
}

#[derive(Serialize, Deserialize)]
pub struct DeleteTableRequest {
  name: String,
}

#[derive(Serialize, Deserialize)]
pub struct DeleteTableResponse {
  name: String,
}
