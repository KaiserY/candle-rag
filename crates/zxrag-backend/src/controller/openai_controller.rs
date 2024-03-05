use axum::response::{sse::Event, IntoResponse, Json, Response, Sse};
use derive_more::{Deref, DerefMut, From};
use either::Either;
use futures::{Stream, StreamExt, TryStream};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use time::OffsetDateTime;
use tinyvec::{tiny_vec, TinyVec};
use uuid::Uuid;
use zxrag_core::llama_cpp::{
  chat_completion_stream, LlamaCppChatCompletionStream, LLAMA_CPP_MODEL,
};
use zxrag_core::model::ChatCompletionSetting;

use crate::error::BackendError;

pub async fn chat_completions(
  Json(req): Json<ChatCompletionRequest<'_>>,
) -> Result<impl IntoResponse, BackendError> {
  let untokenized_context = format!("{}<|ASSISTANT|>", req.messages);

  let mut args = CompletionArgs {
    prompt: untokenized_context,
    seed: req.seed,
    ..Default::default()
  };

  if let Some(one_shot) = req.one_shot {
    args.one_shot = one_shot;
  }

  if let Some(frequency_penalty) = req.frequency_penalty {
    args.frequency_penalty = frequency_penalty;
  }

  let fp = format!("zxrag-{}", "0.1.0");

  let model = (*LLAMA_CPP_MODEL.get().expect("")).clone();

  let setting = ChatCompletionSetting {
    temperature: 0.8,
    top_p: None,
    seed: 299792458,
    repeat_penalty: 1.1,
    repeat_last_n: 64,
    sample_len: 128,
    prompt: None,
    one_shot: false,
  };

  let stream = LlamaCppChatCompletionStream { model, setting };

  let completions_stream = chat_completion_stream(stream).await?.map(move |chunk| {
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
      system_fingerprint: Cow::Borrowed(&fp), // use macro for version
      object: Cow::Borrowed("text_completion"),
    })
  });

  let response = ChatCompletionResponse::Stream(Sse::new(completions_stream));

  Ok(response)
  // Ok(())
}

#[derive(Debug, Clone)]
pub struct CompletionArgs {
  pub prompt: String,
  pub one_shot: bool,
  pub seed: Option<u32>,
  pub frequency_penalty: f32,
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
  pub logit_bias: Option<HashMap<u32, f32>>,
  pub max_tokens: Option<u32>,
  pub n: Option<f32>,
  pub presence_penalty: Option<f32>,
  pub seed: Option<u32>,
  #[serde(default, with = "either::serde_untagged_optional")]
  pub stop: Option<Either<Cow<'a, str>, Vec<Cow<'a, str>>>>,
  pub stream: Option<bool>,
  pub response_format: Option<serde_json::Value>,
  pub temperature: Option<f32>,
  pub top_p: Option<f32>,
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
  /// A unique identifier for this completion.
  pub id: Cow<'a, str>,

  /// The tokens generated by the model.
  pub choices: Vec<ChatCompletionChoice<'a>>,

  /// The UNIX timestamp at which the completion was generated.
  pub created: i64,

  /// The model that generated the completion.
  pub model: Cow<'a, str>,

  /// A unique identifier for the backend configuration that generated the completion.
  pub system_fingerprint: Cow<'a, str>,

  /// The object type. This is always `text_completion`.
  pub object: Cow<'a, str>,

  /// Usage information about this completion.
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
  pub completion_tokens: u32,
  pub prompt_tokens: u32,
  pub total_tokens: u32,
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
  pub index: u32,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ChatCompletionChunkDelta<'a> {
  pub content: Option<Cow<'a, str>>,
  pub role: Option<Cow<'a, str>>,
}
