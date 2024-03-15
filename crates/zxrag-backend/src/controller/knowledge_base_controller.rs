use arrow_array::RecordBatchIterator;
use axum::extract::{Multipart, Path, State};
use axum::response::{sse::Event, IntoResponse, Json, Sse};
use opendal::services::Fs;
use opendal::Operator;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;
use tinyvec::tiny_vec;
use tokio_stream::StreamExt;
use uuid::Uuid;
use zxrag_core::types::handle::{get_embedding_model, get_text_gen};
use zxrag_core::types::lancedb::get_embedding_schema;
use zxrag_core::types::llm::{TextGenerationSetting, TextGenerationStream};

use crate::error::BackendError;
use crate::types::openai::*;
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
  State(state): State<BackendState>,
  Path(_table_id): Path<String>,
  Json(req): Json<ChatCompletionRequest<'_>>,
) -> Result<impl IntoResponse, BackendError> {
  let fp = format!("zxrag-{}", env!("CARGO_PKG_VERSION"));

  let untokenized_context = req.messages.to_prompt(state.config.llm_conf.model_id)?;

  let _last_message = req
    .messages
    .last()
    .ok_or(anyhow::anyhow!("messages is empty"))?;

  let text_gen_setting = TextGenerationSetting {
    temperature: req.temperature.unwrap_or(0.8),
    top_p: req.top_p,
    seed: req.seed.unwrap_or(299792458),
    repeat_penalty: req.frequency_penalty.unwrap_or(1.1),
    repeat_last_n: 64,
    sample_len: req
      .max_tokens
      .map_or(128, |value| value.try_into().unwrap_or(128)),
    prompt: untokenized_context,
  };

  let mut text_gen = get_text_gen(text_gen_setting)?;

  let stream_response = req.stream.unwrap_or(false);

  let response = if stream_response {
    let stream = TextGenerationStream::new(text_gen)?.throttle(Duration::from_millis(10));

    let completions_stream = stream.map(move |chunk| {
      Event::default().json_data(ChatCompletionChunk {
        id: Uuid::new_v4().to_string().into(),
        choices: tiny_vec![ChatCompletionChunkChoice {
          index: 0,
          finish_reason: None,
          delta: ChatCompletionChunkDelta {
            content: Some(Cow::Owned(chunk)),
            role: None,
          },
        }],
        created: OffsetDateTime::now_utc().unix_timestamp(),
        model: Cow::Borrowed("main"),
        system_fingerprint: Cow::Borrowed(&fp),
        object: Cow::Borrowed("text_completion"),
      })
    });

    ChatCompletionResponse::Stream(Sse::new(completions_stream))
  } else {
    let content_str = text_gen.generate()?;

    let response = ChatCompletion {
      id: Uuid::new_v4().to_string().into(),
      choices: vec![ChatCompletionChoice {
        message: ChatMessage::Assistant {
          content: Some(Cow::Owned(content_str)),
          name: None,
          tool_calls: None,
        },
        finish_reason: None,
        index: 0,
      }],
      created: OffsetDateTime::now_utc().unix_timestamp(),
      model: Cow::Borrowed("main"),
      object: Cow::Borrowed("text_completion"),
      system_fingerprint: Cow::Owned(fp),
      usage: ChatCompletionUsage {
        completion_tokens: 0,
        prompt_tokens: 0,
        total_tokens: 0,
      },
    };

    ChatCompletionResponse::Full(Json(response))
  };

  Ok(response)
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
