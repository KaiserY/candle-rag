use axum::response::{sse::Event, IntoResponse, Json, Response, Sse};
use derive_more::{Deref, DerefMut, From};
use either::Either;
use futures::{Stream, TryStream};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write;
use std::fmt::{Display, Formatter};
use std::time::Duration;
use time::OffsetDateTime;
use tinyvec::{tiny_vec, TinyVec};
use tokio_stream::StreamExt;
use uuid::Uuid;
use zxrag_core::models::llama_cpp::{TextGeneration, TextGenerationStream, MODEL};
use zxrag_core::types::conf::ChatCompletionSetting;
use zxrag_core::types::model::ModelId;

use crate::error::BackendError;

pub async fn chat_completions(
  Json(req): Json<ChatCompletionRequest<'_>>,
) -> Result<impl IntoResponse, BackendError> {
  let fp = format!("zxrag-{}", env!("CARGO_PKG_VERSION"));

  let model = (*MODEL.get().ok_or(anyhow::anyhow!("model get error"))?).clone();

  let untokenized_context = req.messages.to_prompt(model.model_id)?;

  let setting = ChatCompletionSetting {
    temperature: req.temperature.unwrap_or(0.8),
    top_p: req.top_p,
    seed: req.seed.unwrap_or(299792458),
    repeat_penalty: req.frequency_penalty.unwrap_or(1.1),
    repeat_last_n: 64,
    sample_len: req
      .max_tokens
      .map_or(128, |value| value.try_into().unwrap_or(128)),
    prompt: Some(untokenized_context),
  };

  let stream_response = req.stream.unwrap_or(false);

  let response = if stream_response {
    let stream = TextGenerationStream::new(TextGeneration::new(model, setting)?)?
      .throttle(Duration::from_millis(10));

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
    let content_str = TextGeneration::new(model, setting)?.generate()?;

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

#[derive(Debug, Clone)]
pub struct CompletionArgs {
  pub prompt: String,
  pub one_shot: bool,
  pub seed: Option<u64>,
  pub frequency_penalty: f64,
}

impl Default for CompletionArgs {
  fn default() -> Self {
    Self {
      prompt: "".to_string(),
      one_shot: false,
      seed: None,
      frequency_penalty: 0.0,
    }
  }
}

#[derive(Serialize, Deserialize)]
pub struct ChatCompletionRequest<'a> {
  #[serde(default)]
  pub messages: ChatMessages<'a>,
  pub model: Cow<'a, str>,
  pub frequency_penalty: Option<f32>,
  pub logit_bias: Option<HashMap<u64, f64>>,
  pub max_tokens: Option<u64>,
  pub n: Option<f64>,
  pub presence_penalty: Option<f64>,
  pub seed: Option<u64>,
  #[serde(default, with = "either::serde_untagged_optional")]
  pub stop: Option<Either<Cow<'a, str>, Vec<Cow<'a, str>>>>,
  pub stream: Option<bool>,
  pub response_format: Option<serde_json::Value>,
  pub temperature: Option<f64>,
  pub top_p: Option<f64>,
  pub tools: Option<Vec<ToolStub<'a>>>,
  #[serde(default, with = "either::serde_untagged_optional")]
  pub tool_choice: Option<Either<Cow<'a, str>, ToolStub<'a>>>,
  pub user: Option<Cow<'a, str>>,
  pub one_shot: Option<bool>,
}

#[derive(Serialize, Deserialize, Default, Deref, DerefMut, From)]
pub struct ChatMessages<'a>(
  #[deref]
  #[deref_mut]
  Vec<ChatMessage<'a>>,
);

impl<'a> ChatMessages<'a> {
  fn to_prompt(&self, model_id: ModelId) -> anyhow::Result<String> {
    let mut prompt = String::new();

    for (i, message) in self.0.iter().enumerate() {
      match message {
        ChatMessage::System {
          content: Some(data),
          ..
        } => {
          if i == 0 {
            write!(prompt, "<s>")?;
          }

          if model_id.is_mistral() {
            write!(
              prompt,
              "[INST] {data} Hi [/INST] Hello! how can I help you</s>"
            )?;
          } else {
            write!(prompt, "<|SYSTEM|>{data}")?;
          }
        }
        ChatMessage::User {
          content: Either::Left(data),
          ..
        } => {
          if i == 0 {
            write!(prompt, "<s>")?;
          }

          if model_id.is_mistral() {
            write!(prompt, "[INST] {data} [/INST]")?;
          } else {
            write!(prompt, "<|USER|>{data}")?;
          }
        }
        ChatMessage::User {
          content: Either::Right(data),
          ..
        } => {
          if i == 0 {
            write!(prompt, "<s>")?;
          }

          if model_id.is_mistral() {
            write!(prompt, "[INST] ")?;

            for part in data {
              write!(prompt, "{part}")?;
            }

            write!(prompt, " [/INST]")?;
          } else {
            write!(prompt, "<|USER|>")?;

            for part in data {
              write!(prompt, "{part}")?;
            }
          }
        }
        ChatMessage::Assistant {
          content: Some(data),
          ..
        } => {
          if model_id.is_mistral() {
            write!(prompt, "{data}")?;
          } else {
            write!(prompt, "<|ASSISTANT|>{data}")?;
          }
        }
        ChatMessage::Tool {
          content: Some(data),
          ..
        } => {
          write!(prompt, "<|TOOL|>{data}")?;
        }
        _ => {}
      }
    }

    Ok(prompt)
  }
}

impl<'a> Display for ChatMessages<'a> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    for message in &self.0 {
      match message {
        ChatMessage::System {
          content: Some(data),
          ..
        } => {
          write!(f, "<|SYSTEM|>{data}")?;
        }
        ChatMessage::User {
          content: Either::Left(data),
          ..
        } => {
          write!(f, "<|USER|>{data}")?;
        }
        ChatMessage::User {
          content: Either::Right(data),
          ..
        } => {
          write!(f, "<|USER|>")?;

          for part in data {
            write!(f, "{part}")?;
          }
        }
        ChatMessage::Assistant {
          content: Some(data),
          ..
        } => {
          write!(f, "<|ASSISTANT|>{data}")?;
        }
        ChatMessage::Tool {
          content: Some(data),
          ..
        } => {
          write!(f, "<|TOOL|>{data}")?;
        }
        _ => {}
      }
    }

    Ok(())
  }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum ChatMessage<'a> {
  #[serde(rename = "system")]
  System {
    content: Option<Cow<'a, str>>,
    name: Option<Cow<'a, str>>,
  },
  #[serde(rename = "user")]
  User {
    #[serde(with = "either::serde_untagged")]
    content: Either<Cow<'a, str>, Vec<ContentPart<'a>>>,
    name: Option<Cow<'a, str>>,
  },

  #[serde(rename = "assistant")]
  Assistant {
    content: Option<Cow<'a, str>>,
    name: Option<Cow<'a, str>>,
    tool_calls: Option<Vec<AssistantToolCall<'a>>>,
  },

  #[serde(rename = "tool")]
  Tool {
    content: Option<Cow<'a, str>>,
    tool_call_id: Cow<'a, str>,
  },
}

#[derive(Serialize, Deserialize)]
pub struct AssistantToolCall<'a> {
  pub id: Cow<'a, str>,
  #[serde(rename = "type")]
  pub type_: Cow<'a, str>,
  pub function: AssistantFunctionStub<'a>,
}

#[derive(Serialize, Deserialize)]
pub struct AssistantFunctionStub<'a> {
  pub name: Cow<'a, str>,
  pub arguments: Cow<'a, str>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart<'a> {
  #[serde(rename = "text")]
  Text { text: Cow<'a, str> },
  #[serde(rename = "image_url")]
  ImageUrl {
    url: Cow<'a, str>,
    detail: Option<Cow<'a, str>>,
  },
}

impl<'a> Display for ContentPart<'a> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      ContentPart::Text { text } => write!(f, "{}", text),
      ContentPart::ImageUrl { url, detail } => {
        if let Some(detail) = detail {
          write!(f, "<IMAGE {}> ({})", url, detail)
        } else {
          write!(f, "<IMAGE {}>", url)
        }
      }
    }
  }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum ToolStub<'a> {
  #[serde(rename = "function")]
  Function { function: FunctionStub<'a> },
}

#[derive(Serialize, Deserialize)]
pub struct FunctionStub<'a> {
  pub description: Option<Cow<'a, str>>,
  pub name: Cow<'a, str>,
  pub parameters: serde_json::Value,
}

enum ChatCompletionResponse<'a, S>
where
  S: TryStream<Ok = Event> + Send + 'static,
{
  Stream(Sse<S>),
  Full(Json<ChatCompletion<'a>>),
}

impl<'a, S, E> IntoResponse for ChatCompletionResponse<'a, S>
where
  S: Stream<Item = Result<Event, E>> + Send + 'static,
  E: Into<axum::BoxError>,
{
  fn into_response(self) -> Response {
    match self {
      ChatCompletionResponse::Stream(stream) => stream.into_response(),
      ChatCompletionResponse::Full(full) => full.into_response(),
    }
  }
}

#[derive(Serialize, Deserialize)]
pub struct ChatCompletion<'a> {
  pub id: Cow<'a, str>,
  pub choices: Vec<ChatCompletionChoice<'a>>,
  pub created: i64,
  pub model: Cow<'a, str>,
  pub system_fingerprint: Cow<'a, str>,
  pub object: Cow<'a, str>,
  pub usage: ChatCompletionUsage,
}

#[derive(Serialize, Deserialize)]
pub struct ChatCompletionChoice<'a> {
  pub message: ChatMessage<'a>,
  pub finish_reason: Option<Cow<'a, str>>,
  pub index: i32,
}

#[derive(Serialize, Deserialize)]
pub struct ChatCompletionUsage {
  pub completion_tokens: u64,
  pub prompt_tokens: u64,
  pub total_tokens: u64,
}

#[derive(Serialize, Deserialize)]
pub struct ChatCompletionChunk<'a> {
  pub id: Cow<'a, str>,
  pub choices: TinyVec<[ChatCompletionChunkChoice<'a>; 1]>,
  pub created: i64,
  pub model: Cow<'a, str>,
  pub system_fingerprint: Cow<'a, str>,
  pub object: Cow<'a, str>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ChatCompletionChunkChoice<'a> {
  pub delta: ChatCompletionChunkDelta<'a>,
  pub finish_reason: Option<Cow<'a, str>>,
  pub index: u64,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ChatCompletionChunkDelta<'a> {
  pub content: Option<Cow<'a, str>>,
  pub role: Option<Cow<'a, str>>,
}
