use axum::response::{sse::Event, IntoResponse, Json, Response, Sse};
use derive_more::{Deref, DerefMut, From};
use either::Either;
use futures::{Stream, TryStream};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write;
use std::fmt::{Display, Formatter};
use tinyvec::TinyVec;

use zxrag_core::types::model::ModelId;

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
  pub fn to_prompt(&self, model_id: ModelId) -> anyhow::Result<String> {
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

pub enum ChatCompletionResponse<'a, S>
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

#[derive(Serialize, Deserialize)]
pub struct CreateEmbeddingRequest<'a> {
  #[serde(with = "either::serde_untagged")]
  pub input: Either<Cow<'a, str>, Vec<Cow<'a, str>>>,
  pub model: Cow<'a, str>,
  pub encoding_format: Option<Cow<'a, str>>,
  pub dimensions: Option<usize>,
}

#[derive(Serialize, Deserialize)]
pub struct EmbeddingResponse {
  pub object: String,
  pub embeddings: Vec<Embedding>,
  pub model: String,
  pub usage: EmbeddingsUsage,
}

#[derive(Serialize, Deserialize)]
pub struct Embedding {
  pub object: String,
  pub embedding: Vec<f32>,
  pub index: usize,
}

#[derive(Serialize, Deserialize)]
pub struct EmbeddingsUsage {
  pub prompt_tokens: usize,
  pub total_tokens: usize,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct ModelsResponse {
  pub object: String,
  pub data: Vec<ModelDescription>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct ModelDescription {
  pub id: String,
  pub created: u64,
  pub object: String,
  pub owned_by: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct ListFilesResponse<'a> {
  pub object: String,
  pub data: Vec<File<'a>>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct File<'a> {
  pub id: Cow<'a, str>,
  pub bytes: u64,
  pub created_at: u64,
  pub filename: Cow<'a, str>,
  pub object: Cow<'a, str>,
  pub purpose: Cow<'a, str>,
}
