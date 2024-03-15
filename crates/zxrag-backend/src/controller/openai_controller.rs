use axum::extract::{Multipart, Path, State};
use axum::response::{sse::Event, IntoResponse, Json, Sse};
use opendal::services::Fs;
use opendal::Operator;
use std::borrow::Cow;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;
use tinyvec::tiny_vec;
use tokio_stream::StreamExt;
use uuid::Uuid;
use zxrag_core::types::handle::{get_embedding_model, get_text_gen};
use zxrag_core::types::llm::{TextGenerationSetting, TextGenerationStream};

use crate::error::BackendError;
use crate::types::openai::*;
use crate::BackendState;

pub async fn create_chat_completion(
  State(state): State<BackendState>,
  Json(req): Json<ChatCompletionRequest<'_>>,
) -> Result<impl IntoResponse, BackendError> {
  let fp = format!("zxrag-{}", env!("CARGO_PKG_VERSION"));

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

pub async fn embeddings(
  State(state): State<BackendState>,
  Json(req): Json<CreateEmbeddingRequest<'_>>,
) -> Result<impl IntoResponse, BackendError> {
  let input = req.input.either(
    move |s| vec![s.to_string()],
    move |v| v.iter().map(move |s| s.to_string()).collect(),
  );

  let bert_model = get_embedding_model(state.config.embedding_conf.model_id)?;

  let prompts: Vec<&str> = input.iter().map(|s| s.as_str()).collect();

  let mut embeddings: Vec<Vec<f32>> = bert_model.embedding_batch(&prompts)?;

  Ok(Json(EmbeddingResponse {
    object: "list".to_string(),
    embeddings: embeddings
      .drain(..)
      .enumerate()
      .map(move |(index, embedding)| Embedding {
        object: "embedding".to_string(),
        embedding,
        index,
      })
      .collect(),
    model: req.model.to_string(),
    usage: EmbeddingsUsage {
      prompt_tokens: 0,
      total_tokens: 0,
    },
  }))
}

pub async fn models(State(state): State<BackendState>) -> Result<impl IntoResponse, BackendError> {
  let models = vec![
    ModelDescription {
      id: state.config.llm_conf.model_id.to_string(),
      created: SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs(),
      object: "model".to_string(),
      owned_by: "llm".to_string(),
    },
    ModelDescription {
      id: state.config.embedding_conf.model_id.to_string(),
      created: SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs(),
      object: "model".to_string(),
      owned_by: "embedding".to_string(),
    },
  ];

  Ok(Json(ModelsResponse {
    object: "list".to_string(),
    data: models,
  }))
}

pub async fn upload_file(
  State(state): State<BackendState>,
  mut multipart: Multipart,
) -> Result<impl IntoResponse, BackendError> {
  while let Some(mut field) = multipart
    .next_field()
    .await
    .map_err(|e| anyhow::anyhow!(e))?
  {
    let name = field.name().unwrap_or_default();

    tracing::info!("name={}", name);

    if name == "file" {
      let file_name = field
        .file_name()
        .ok_or(anyhow::anyhow!("file_name not found"))?;

      let mut builder = Fs::default();

      builder.root(&state.config.opendal_path);

      let op: Operator = Operator::new(builder)
        .map_err(|e| anyhow::anyhow!(e))?
        .finish();

      let mut w = op
        .writer_with(file_name)
        .append(true)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

      while let Some(chunk) = field.chunk().await.map_err(|e| anyhow::anyhow!(e))? {
        w.write(chunk).await.map_err(|e| anyhow::anyhow!(e))?;
      }

      w.close().await.map_err(|e| anyhow::anyhow!(e))?;
    }
  }

  Ok(())
}

pub async fn list_files(
  State(state): State<BackendState>,
) -> Result<impl IntoResponse, BackendError> {
  let mut builder = Fs::default();

  builder.root(&state.config.opendal_path);

  let op: Operator = Operator::new(builder)
    .map_err(|e| anyhow::anyhow!(e))?
    .finish();

  let files = op.list("/").await.map_err(|e| anyhow::anyhow!(e))?;

  Ok(Json(ListFilesResponse {
    object: "list".to_string(),
    data: files
      .into_iter()
      .map(|f| File {
        id: Cow::Owned(f.name().to_string()),
        bytes: f.metadata().content_length(),
        created_at: SystemTime::now()
          .duration_since(UNIX_EPOCH)
          .unwrap_or_default()
          .as_secs(),
        filename: Cow::Owned(f.name().to_string()),
        object: Cow::Owned("file".to_string()),
        purpose: Cow::Owned("embeddings".to_string()),
      })
      .collect(),
  }))
}

pub async fn delete_file(
  State(state): State<BackendState>,
  Path(file_id): Path<String>,
) -> Result<impl IntoResponse, BackendError> {
  let mut builder = Fs::default();

  builder.root(&state.config.opendal_path);

  let op: Operator = Operator::new(builder)
    .map_err(|e| anyhow::anyhow!(e))?
    .finish();

  op.delete(&file_id).await.map_err(|e| anyhow::anyhow!(e))?;

  Ok(())
}
