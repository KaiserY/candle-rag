use arrow_array::types::Float32Type;
use arrow_array::{
  FixedSizeListArray, Int64Array, PrimitiveArray, RecordBatch, RecordBatchIterator, StringArray,
};
use axum::extract::{Multipart, Path, State};
use axum::response::{sse::Event, IntoResponse, Json, Sse};
use futures::TryStreamExt;
use opendal::services::Fs;
use opendal::Operator;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tinyvec::tiny_vec;
use tokio_stream::StreamExt;
use uuid::Uuid;
use zxrag_core::types::handle::{get_embedding_model, get_text_gen};
use zxrag_core::types::knowledge_base::{Embedding, EmbeddingResponse, EmbeddingsUsage};
use zxrag_core::types::lancedb::get_embedding_schema;
use zxrag_core::types::llm::{TextGenerationSetting, TextGenerationStream};
use zxrag_core::types::openai::{
  ChatCompletion, ChatCompletionChoice, ChatCompletionChunk, ChatCompletionChunkChoice,
  ChatCompletionChunkDelta, ChatCompletionRequest, ChatCompletionUsage, ChatMessage,
  DeleteFileResponse, File, ListFilesResponse,
};
use zxrag_core::types::sqlx::File as SqlxFile;
use zxrag_core::types::sqlx::KnowledgeBase;

use crate::error::BackendError;
use crate::openai_controller::ChatCompletionResponse;
use crate::BackendState;

pub async fn create_knowledge_base(
  State(state): State<BackendState>,
  Json(req): Json<CreateKnowledgeBaseRequest>,
) -> Result<impl IntoResponse, BackendError> {
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

  let knowledge_base = sqlx::query_as::<_, KnowledgeBase>(
    r#"
SELECT * FROM knowledge_base where name = ?;
    "#,
  )
  .bind(&req.name)
  .fetch_one(&state.pool.clone())
  .await
  .map_err(|e| anyhow::anyhow!(e))?;

  let db = vectordb::connect(&state.config.lancedb_path)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  let tables = db.table_names().await.map_err(|e| anyhow::anyhow!(e))?;

  let kb_table_name = format!("kb_{}", knowledge_base.id);

  if !tables.contains(&kb_table_name) {
    let schema = get_embedding_schema()?;

    let batches = RecordBatchIterator::new(vec![], schema);

    db.create_table(&kb_table_name, Box::new(batches), None)
      .await
      .map_err(|e| anyhow::anyhow!(e))?;
  }

  Ok(Json(CreateKnowledgeBaseResponse {
    id: knowledge_base.id,
    name: req.name,
    existed: false,
  }))
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

  let kb_table_name = format!("kb_{}", knowledge_base.id);

  let db = vectordb::connect(&state.config.lancedb_path)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  let tables = db.table_names().await.map_err(|e| anyhow::anyhow!(e))?;

  if tables.contains(&kb_table_name) {
    db.drop_table(&kb_table_name)
      .await
      .map_err(|e| anyhow::anyhow!(e))?;
  }

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

pub async fn upload_file(
  State(state): State<BackendState>,
  Path(kb_id): Path<String>,
  mut multipart: Multipart,
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

  let kb_table_name = format!("kb_{}", knowledge_base.id);

  while let Some(mut field) = multipart
    .next_field()
    .await
    .map_err(|e| anyhow::anyhow!(e))?
  {
    let name = field.name().unwrap_or_default();

    if name == "file" {
      let file_name = field
        .file_name()
        .ok_or(anyhow::anyhow!("file_name not found"))?
        .to_string();

      let mut builder = Fs::default();

      builder.root(&format!("{}/{}", &state.config.opendal_path, kb_table_name));

      let op: Operator = Operator::new(builder)
        .map_err(|e| anyhow::anyhow!(e))?
        .finish();

      let mut w = op
        .writer_with(&file_name)
        .append(true)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

      let mut bytes: i64 = 0;

      while let Some(chunk) = field.chunk().await.map_err(|e| anyhow::anyhow!(e))? {
        bytes += chunk.len() as i64;

        w.write(chunk).await.map_err(|e| anyhow::anyhow!(e))?;
      }

      w.close().await.map_err(|e| anyhow::anyhow!(e))?;

      sqlx::query(
        r#"
REPLACE INTO file ( kb_id, filename, bytes, purpose, created_at, updated_at )
VALUES ( ?, ?, ?, ?, ?, ? );
        "#,
      )
      .bind(knowledge_base.id)
      .bind(&file_name)
      .bind(bytes)
      .bind("embedding")
      .bind(OffsetDateTime::now_utc().unix_timestamp())
      .bind(OffsetDateTime::now_utc().unix_timestamp())
      .execute(&state.pool.clone())
      .await
      .map_err(|e| anyhow::anyhow!(e))?;
    }
  }

  Ok(())
}

pub async fn list_files(
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

  let sqlx_files = sqlx::query_as::<_, SqlxFile>(
    r#"
SELECT * FROM file WHERE kb_id = ?;
    "#,
  )
  .bind(knowledge_base.id)
  .fetch_all(&state.pool.clone())
  .await
  .map_err(|e| anyhow::anyhow!(e))?;

  Ok(Json(ListFilesResponse {
    object: Cow::Owned("list".to_string()),
    data: sqlx_files
      .into_iter()
      .map(|f| File {
        id: Cow::Owned(f.id.to_string()),
        bytes: f.bytes,
        created_at: f.created_at,
        filename: Cow::Owned(f.filename),
        object: Cow::Owned("file".to_string()),
        purpose: Cow::Owned(f.purpose),
      })
      .collect(),
  }))
}

pub async fn delete_file(
  State(state): State<BackendState>,
  Path((kb_id, file_id)): Path<(String, String)>,
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

  let kb_table_name = format!("kb_{}", knowledge_base.id);

  let sqlx_file = sqlx::query_as::<_, SqlxFile>(
    r#"
SELECT * FROM file where id = ? AND kb_id = ?;
    "#,
  )
  .bind(file_id)
  .bind(knowledge_base.id)
  .fetch_one(&state.pool.clone())
  .await
  .map_err(|e| anyhow::anyhow!(e))?;

  let mut builder = Fs::default();

  builder.root(&format!("{}/{}", &state.config.opendal_path, kb_table_name));

  let op: Operator = Operator::new(builder)
    .map_err(|e| anyhow::anyhow!(e))?
    .finish();

  op.delete(&sqlx_file.filename)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  sqlx::query(
    r#"
DELETE FROM file where id = ? AND kb_id = ?;
    "#,
  )
  .bind(sqlx_file.id)
  .bind(knowledge_base.id)
  .execute(&state.pool.clone())
  .await
  .map_err(|e| anyhow::anyhow!(e))?;

  Ok(Json(DeleteFileResponse {
    id: Cow::Owned(sqlx_file.id.to_string()),
    object: Cow::Owned("file".to_string()),
    deleted: true,
  }))
}

pub async fn create_embeddings(
  State(state): State<BackendState>,
  Path((kb_id, file_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, BackendError> {
  let knowledge_base = sqlx::query_as::<_, KnowledgeBase>(
    r#"
SELECT * FROM knowledge_base where id = ?;
    "#,
  )
  .bind(&kb_id)
  .fetch_one(&state.pool.clone())
  .await
  .map_err(|e| anyhow::anyhow!(e))?;

  let kb_table_name = format!("kb_{}", knowledge_base.id);

  let sqlx_file = sqlx::query_as::<_, SqlxFile>(
    r#"
SELECT * FROM file where id = ? AND kb_id = ?;
    "#,
  )
  .bind(&file_id)
  .bind(&kb_id)
  .fetch_one(&state.pool.clone())
  .await
  .map_err(|e| anyhow::anyhow!(e))?;

  let mut builder = Fs::default();

  builder.root(&format!("{}/{}", &state.config.opendal_path, kb_table_name));

  let op: Operator = Operator::new(builder)
    .map_err(|e| anyhow::anyhow!(e))?
    .finish();

  let file_bytes = op
    .read(&sqlx_file.filename)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  let bert_model = get_embedding_model(state.config.embedding_conf.model_id)?;

  let prompts: Vec<&str> = vec![std::str::from_utf8(&file_bytes).map_err(|e| anyhow::anyhow!(e))?];

  let embeddings: Vec<Vec<f32>> = bert_model.embedding_batch(&prompts)?;

  let vectors: Vec<Option<Vec<Option<f32>>>> = embeddings
    .into_iter()
    .map(|t| Some(t.into_iter().map(Some).collect()))
    .collect();

  let schema = get_embedding_schema()?;

  let db = vectordb::connect(&state.config.lancedb_path)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  let tbl = db
    .open_table(&kb_table_name)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  let uuid_strings: Vec<Option<String>> = (0..vectors.len())
    .map(|_| Some(Uuid::new_v4().to_string()))
    .collect();

  let uuid_str_slices: Vec<Option<&str>> =
    uuid_strings.iter().map(|uuid| uuid.as_deref()).collect();

  let batches = RecordBatchIterator::new(
    vec![RecordBatch::try_new(
      schema.clone(),
      vec![
        Arc::new(StringArray::from(uuid_str_slices)),
        Arc::new(Int64Array::from_iter_values(
          (0..vectors.len()).map(|_| knowledge_base.id),
        )),
        Arc::new(Int64Array::from_iter_values(
          (0..vectors.len()).map(|_| sqlx_file.id),
        )),
        Arc::new(StringArray::from(
          (0..vectors.len())
            .map(|_| Some(sqlx_file.filename.as_str()))
            .collect::<Vec<Option<&str>>>(),
        )),
        Arc::new(StringArray::from(
          prompts.into_iter().map(Some).collect::<Vec<Option<&str>>>(),
        )),
        Arc::new(FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(vectors, 1024)),
      ],
    )
    .map_err(|e| anyhow::anyhow!(e))?]
    .into_iter()
    .map(Ok),
    schema.clone(),
  );

  tbl
    .add(Box::new(batches), None)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  Ok(())
}

pub async fn list_embeddings(
  State(state): State<BackendState>,
  Path(kb_id): Path<String>,
) -> Result<impl IntoResponse, BackendError> {
  let knowledge_base = sqlx::query_as::<_, KnowledgeBase>(
    r#"
SELECT * FROM knowledge_base where id = ?;
    "#,
  )
  .bind(&kb_id)
  .fetch_one(&state.pool.clone())
  .await
  .map_err(|e| anyhow::anyhow!(e))?;

  let kb_table_name = format!("kb_{}", knowledge_base.id);

  let db = vectordb::connect(&state.config.lancedb_path)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  let tbl = db
    .open_table(&kb_table_name)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  let result = tbl
    .query()
    .filter(format!("kb_id = {}", &knowledge_base.id))
    .execute_stream()
    .await
    .map_err(|e| anyhow::anyhow!(e))?
    .try_collect::<Vec<_>>()
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  Ok(Json(EmbeddingResponse {
    object: Cow::Owned("list".to_string()),
    embeddings: result
      .into_iter()
      .enumerate()
      .map(|(index, batch)| {
        let embedding_id: StringArray = batch.column(0).to_data().into();
        let kb_id: Int64Array = batch.column(1).to_data().into();
        let file_id: Int64Array = batch.column(2).to_data().into();
        let filename: StringArray = batch.column(3).to_data().into();
        let text: StringArray = batch.column(4).to_data().into();
        let vector: FixedSizeListArray = batch.column(5).to_data().into();

        Embedding {
          id: Cow::Owned(embedding_id.value(0).to_string()),
          kb_id: kb_id.value(0),
          file_id: file_id.value(0),
          filename: Cow::Owned(filename.value(0).to_string()),
          object: Cow::Owned("embedding".to_string()),
          text: Cow::Owned(text.value(0).to_string()),
          embedding: Into::<PrimitiveArray<Float32Type>>::into(vector.value(0).to_data())
            .values()
            .to_vec(),
          index,
        }
      })
      .collect(),
    model: Cow::Owned(state.config.embedding_conf.model_id.to_string()),
    usage: EmbeddingsUsage {
      prompt_tokens: 0,
      total_tokens: 0,
    },
  }))
}

pub async fn delete_embeddings(
  State(state): State<BackendState>,
  Path((kb_id, embedding_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, BackendError> {
  let knowledge_base = sqlx::query_as::<_, KnowledgeBase>(
    r#"
SELECT * FROM knowledge_base where id = ?;
    "#,
  )
  .bind(&kb_id)
  .fetch_one(&state.pool.clone())
  .await
  .map_err(|e| anyhow::anyhow!(e))?;

  let kb_table_name = format!("kb_{}", knowledge_base.id);

  let db = vectordb::connect(&state.config.lancedb_path)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  let tbl = db
    .open_table(&kb_table_name)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  tbl
    .delete(&format!("id = '{}'", embedding_id))
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  Ok(())
}

pub async fn create_chat_completion(
  State(state): State<BackendState>,
  Path(kb_id): Path<String>,
  Json(mut req): Json<ChatCompletionRequest<'_>>,
) -> Result<impl IntoResponse, BackendError> {
  let fp = format!("zxrag-{}", env!("CARGO_PKG_VERSION"));

  let knowledge_base = sqlx::query_as::<_, KnowledgeBase>(
    r#"
SELECT * FROM knowledge_base where id = ?;
    "#,
  )
  .bind(&kb_id)
  .fetch_one(&state.pool.clone())
  .await
  .map_err(|e| anyhow::anyhow!(e))?;

  let kb_table_name = format!("kb_{}", knowledge_base.id);

  let db = vectordb::connect(&state.config.lancedb_path)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  let tbl = db
    .open_table(&kb_table_name)
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

  let last_message = req
    .messages
    .last_mut()
    .ok_or(anyhow::anyhow!("messages is empty"))?;

  let bert_model = get_embedding_model(state.config.embedding_conf.model_id)?;

  let last_message_str = last_message.to_string();

  let prompts: Vec<&str> = vec![&last_message_str];

  let embeddings: Vec<Vec<f32>> = bert_model.embedding_batch(&prompts)?;

  let result: Vec<String> = tbl
    .search(&embeddings[0])
    .limit(1)
    .execute_stream()
    .await
    .map_err(|e| anyhow::anyhow!(e))?
    .try_collect::<Vec<_>>()
    .await
    .map_err(|e| anyhow::anyhow!(e))?
    .iter()
    .map(|batch| {
      let text: StringArray = batch.column(4).to_data().into();

      text.value(0).to_string()
    })
    .collect();

  *last_message = ChatMessage::User {
    content: either::Left(Cow::Owned(format!(
      "{} answer question use the follwing information: {}",
      last_message_str,
      result.join("\n")
    ))),
    name: None,
  };

  let untokenized_context = req.messages.to_prompt(state.config.llm_conf.model_id)?;

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
  id: i64,
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
