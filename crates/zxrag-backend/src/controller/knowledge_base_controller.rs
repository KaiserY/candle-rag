use arrow_array::RecordBatchIterator;
use axum::extract::{Path, State};
use axum::response::{sse::Event, IntoResponse, Json, Sse};

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::time::Duration;
use time::OffsetDateTime;
use tinyvec::tiny_vec;
use tokio_stream::StreamExt;
use uuid::Uuid;
use zxrag_core::types::handle::get_text_gen;
use zxrag_core::types::lancedb::get_embedding_schema;
use zxrag_core::types::llm::{TextGenerationSetting, TextGenerationStream};
use zxrag_core::types::openai::*;
use zxrag_core::types::sqlx::KnowledgeBase;

use crate::error::BackendError;
use crate::openai_controller::ChatCompletionResponse;
use crate::BackendState;

pub async fn create_knowledge_base(
  State(state): State<BackendState>,
  Json(req): Json<CreateKnowledgeBaseRequest>,
) -> Result<impl IntoResponse, BackendError> {
  let db = vectordb::connect(&state.config.lancedb_path)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  let tables = db.table_names().await.map_err(|e| anyhow::anyhow!(e))?;

  if tables.contains(&req.name) {
    Ok(Json(CreateKnowledgeBaseResponse {
      name: req.name,
      existed: true,
    }))
  } else {
    let schema = get_embedding_schema()?;

    let batches = RecordBatchIterator::new(vec![], schema);

    let tbl = db
      .create_table(&req.name, Box::new(batches), None)
      .await
      .map_err(|e| anyhow::anyhow!(e))?;

    tracing::info!("{}", tbl);

    sqlx::query(
      r#"
INSERT INTO knowledge_base ( name, created_at, updated_at )
VALUES ( ?, ?, ? );
        "#,
    )
    .bind(&req.name)
    .bind(OffsetDateTime::now_utc().unix_timestamp())
    .bind(OffsetDateTime::now_utc().unix_timestamp())
    .execute(&state.pool.clone())
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

    Ok(Json(CreateKnowledgeBaseResponse {
      name: req.name,
      existed: false,
    }))
  }
}

pub async fn list_knowledge_bases(
  State(state): State<BackendState>,
) -> Result<impl IntoResponse, BackendError> {
  let knowledge_bases = sqlx::query_as::<_, KnowledgeBase>(
    r#"
  SELECT * FROM knowledge_base;
      "#,
  )
  .fetch_all(&state.pool.clone())
  .await
  .map_err(|e| anyhow::anyhow!(e))?;

  Ok(Json(ListKnowledgeBaseResponse {
    data: knowledge_bases,
  }))
}

pub async fn delete_knowledge_base(
  State(state): State<BackendState>,
  Path(kb_id): Path<String>,
) -> Result<impl IntoResponse, BackendError> {
  let knowledge_base = sqlx::query_as::<_, KnowledgeBase>(
    r#"
SELECT * FROM knowledge_base where id = ?;
    "#,
  )
  .bind(kb_id)
  .fetch_one(&state.pool.clone())
  .await
  .map_err(|e| anyhow::anyhow!(e))?;

  sqlx::query(
    r#"
DELETE FROM knowledge_base where id = ?;
    "#,
  )
  .bind(knowledge_base.id)
  .execute(&state.pool.clone())
  .await
  .map_err(|e| anyhow::anyhow!(e))?;

  Ok(Json(DeleteKnowledgeBaseResponse {
    name: knowledge_base.name,
  }))
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
pub struct CreateKnowledgeBaseRequest {
  name: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateKnowledgeBaseResponse {
  name: String,
  existed: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ListKnowledgeBaseResponse {
  data: Vec<KnowledgeBase>,
}

#[derive(Serialize, Deserialize)]
pub struct DeleteKnowledgeBaseResponse {
  name: String,
}
