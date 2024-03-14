use axum::extract::State;
use axum::response::{sse::Event, IntoResponse, Json, Response, Sse};
use derive_more::{Deref, DerefMut, From};
use either::Either;
use futures::{Stream, TryStream};
use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write;
use std::fmt::{Display, Formatter};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;
use tinyvec::{tiny_vec, TinyVec};
use tokio_stream::StreamExt;
use uuid::Uuid;
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
  let db = Database::create(&state.config.redb_path).map_err(|e| anyhow::anyhow!(e))?;

  let read_txn = db.begin_read().map_err(|e| anyhow::anyhow!(e))?;
  let table = read_txn.open_table(TABLE).map_err(|e| anyhow::anyhow!(e))?;

  let aa = table
    .iter()
    .map_err(|e| anyhow::anyhow!(e))?
    .filter(|x| true);

  Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct CreateDatabaseRequest {}
