use arrow_array::{
  types::Float32Type, FixedSizeListArray, Int32Array, RecordBatch, RecordBatchIterator, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use axum::extract::State;
use axum::response::{sse::Event, IntoResponse, Json, Response, Sse};
use derive_more::{Deref, DerefMut, From};
use either::Either;
use futures::{Stream, TryStream};
use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;
use tinyvec::{tiny_vec, TinyVec};
use tokio_stream::StreamExt;
use uuid::Uuid;
use zxrag_core::types::embedding::{get_embedding_schema, EMBEDDING_SCHEMA};
use zxrag_core::types::handle::{get_embedding_model, get_text_gen};
use zxrag_core::types::llm::{TextGenerationSetting, TextGenerationStream};
use zxrag_core::types::model::ModelId;

use crate::error::BackendError;
use crate::BackendState;

const TABLE: TableDefinition<&str, &str> = TableDefinition::new("my_data");

pub async fn create_databases(
  State(state): State<BackendState>,
  Json(req): Json<CreateDatabaseRequest>,
) -> Result<impl IntoResponse, BackendError> {
  let db = vectordb::connect(&state.config.lancedb_path)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  let tables = db.table_names().await.map_err(|e| anyhow::anyhow!(e))?;

  let table_name = "my_table".to_string();

  if tables.contains(&table_name) {
    Ok(Json(CreateDatabaseResponse { name: req.name }))
  } else {
    let schema = get_embedding_schema()?;

    let batches = RecordBatchIterator::new(vec![], schema);

    let tbl = db
      .create_table(&req.name, Box::new(batches), None)
      .await
      .map_err(|e| anyhow::anyhow!(e))?;

    tracing::info!("{}", tbl);

    Ok(Json(CreateDatabaseResponse { name: req.name }))
  }
}

pub async fn list_databases(
  State(state): State<BackendState>,
) -> Result<impl IntoResponse, BackendError> {
  let db = vectordb::connect(&state.config.lancedb_path)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  let tables = db.table_names().await.map_err(|e| anyhow::anyhow!(e))?;

  let data = tables
    .into_iter()
    .map(|t| DatabaseDescription { name: t })
    .collect();

  Ok(Json(ListDatabaseResponse { data }))
}

#[derive(Serialize, Deserialize)]
pub struct CreateDatabaseRequest {
  name: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateDatabaseResponse {
  name: String,
}

#[derive(Serialize, Deserialize)]
pub struct ListDatabaseResponse {
  data: Vec<DatabaseDescription>,
}

#[derive(Serialize, Deserialize)]
pub struct DatabaseDescription {
  name: String,
}
