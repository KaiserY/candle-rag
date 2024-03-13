use candle_core::{Device, Tensor};
use candle_transformers::generation::LogitsProcessor;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokenizers::Tokenizer;

use crate::types::{
  model::{ModelEngine, ModelId},
  token_output_stream::TokenOutputStream,
};
use crate::util::eos_token;

const MAX_SEQ_LEN: usize = 4096;

pub trait LlmModel: Send + Sync {
  fn id(&self) -> ModelId;
  fn engine(&self) -> ModelEngine;
  fn tokenizer(&self) -> &Tokenizer;
  fn device(&self) -> &Device;
  fn forward(&mut self, x: &Tensor, index_pos: usize) -> anyhow::Result<Tensor>;
}

pub struct TextGenerationSetting {
  pub temperature: f64,
  pub top_p: Option<f64>,
  pub seed: u64,
  pub repeat_penalty: f32,
  pub repeat_last_n: usize,
  pub sample_len: usize,
  pub prompt: String,
}

pub struct TextGeneration {
  pub model: Box<dyn LlmModel + Send + Sync>,
  pub setting: TextGenerationSetting,
  logits_processor: LogitsProcessor,
  token_output_stream: TokenOutputStream,
  all_tokens: Vec<u32>,
  eos_token: u32,
}

impl TextGeneration {
  pub fn new(
    model: Box<dyn LlmModel + Send + Sync>,
    setting: TextGenerationSetting,
  ) -> anyhow::Result<Self> {
    let temperature = if setting.temperature == 0. {
      None
    } else {
      Some(setting.temperature)
    };

    let logits_processor = LogitsProcessor::new(setting.seed, temperature, setting.top_p);

    let token_output_stream = TokenOutputStream::new(model.tokenizer().clone());

    let eos_token = eos_token(model.id());

    let eos_token = *token_output_stream
      .tokenizer()
      .get_vocab(true)
      .get(eos_token)
      .ok_or(anyhow::anyhow!("get eos_token failed"))?;

    Ok(Self {
      model,
      setting,
      logits_processor,
      token_output_stream,
      all_tokens: vec![],
      eos_token,
    })
  }

  pub fn generate(&mut self) -> anyhow::Result<String> {
    tracing::info!("prompt={}", self.setting.prompt);

    let tokens = self
      .token_output_stream
      .tokenizer()
      .encode(self.setting.prompt.clone(), true)
      .map_err(anyhow::Error::msg)?;

    let prompt_tokens = tokens.get_ids().to_vec();

    self.all_tokens.extend(prompt_tokens);

    let start_gen = std::time::Instant::now();

    let mut generated_tokens = 0usize;

    let mut output = String::new();

    for index in 0..self.setting.sample_len {
      let context_size = if index > 0 { 1 } else { self.all_tokens.len() };

      let start_pos = self.all_tokens.len().saturating_sub(context_size);

      let next_token = self.forward_token(start_pos)?;

      self.all_tokens.push(next_token);

      generated_tokens += 1;

      if next_token == self.eos_token {
        break;
      };

      if let Some(t) = self.token_output_stream.next_token(next_token)? {
        tracing::info!("t={}", t);

        output.push_str(&t);
      }
    }

    if let Some(rest) = self
      .token_output_stream
      .decode_rest()
      .map_err(candle_core::Error::msg)?
    {
      tracing::info!("rest={}", rest);

      output.push_str(&rest);
    }

    let dt = start_gen.elapsed();
    tracing::info!(
      "\n{generated_tokens} tokens generated ({:.2} token/s)",
      generated_tokens as f64 / dt.as_secs_f64(),
    );

    Ok(output)
  }

  pub fn forward_token(&mut self, index_pos: usize) -> anyhow::Result<u32> {
    let ctxt = &self.all_tokens[index_pos..];

    let input = Tensor::new(ctxt, self.model.device())?.unsqueeze(0)?;

    let logits = self.model.forward(&input, index_pos)?;

    let logits = logits.squeeze(0)?;

    let logits = if self.setting.repeat_penalty == 1. {
      logits
    } else {
      let start_at = self
        .all_tokens
        .len()
        .saturating_sub(self.setting.repeat_last_n);
      candle_transformers::utils::apply_repeat_penalty(
        &logits,
        self.setting.repeat_penalty,
        &self.all_tokens[start_at..],
      )?
    };

    Ok(self.logits_processor.sample(&logits)?)
  }
}

pub struct TextGenerationStream {
  pub text_gen: TextGeneration,
  generated_tokens: usize,
}

impl TextGenerationStream {
  pub fn new(mut text_gen: TextGeneration) -> anyhow::Result<Self> {
    tracing::info!("prompt_str={}", &text_gen.setting.prompt);

    let tokens = text_gen
      .token_output_stream
      .tokenizer()
      .encode(text_gen.setting.prompt.clone(), true)
      .map_err(anyhow::Error::msg)?;

    let mut prompt_tokens = tokens.get_ids().to_owned();

    prompt_tokens = if prompt_tokens.len() + text_gen.setting.sample_len > MAX_SEQ_LEN - 10 {
      let to_remove = prompt_tokens.len() + text_gen.setting.sample_len + 10 - MAX_SEQ_LEN;
      prompt_tokens[prompt_tokens.len().saturating_sub(to_remove)..].to_vec()
    } else {
      prompt_tokens
    };

    text_gen.all_tokens.extend(prompt_tokens.clone());

    Ok(Self {
      text_gen,
      generated_tokens: 0,
    })
  }
}

impl Stream for TextGenerationStream {
  type Item = String;

  fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    tracing::info!("generated_tokens={}", self.generated_tokens);

    if self.generated_tokens > self.text_gen.setting.sample_len {
      return Poll::Ready(None);
    }

    let context_size = if self.generated_tokens > 0 {
      1
    } else {
      self.text_gen.all_tokens.len()
    };

    let start_pos = self.text_gen.all_tokens.len().saturating_sub(context_size);

    let next_token = self.text_gen.forward_token(start_pos);

    match next_token {
      Ok(next_token) => {
        self.text_gen.all_tokens.push(next_token);
        self.generated_tokens += 1;

        tracing::info!("next_token={}", next_token);

        if next_token == self.text_gen.eos_token {
          Poll::Ready(None)
        } else if let Ok(t) = self.text_gen.token_output_stream.next_token(next_token) {
          tracing::info!("t={:?}", t);

          if let Some(t) = t {
            Poll::Ready(Some(t))
          } else {
            Poll::Ready(Some("".to_string()))
          }
        } else {
          Poll::Ready(None)
        }
      }
      Err(_) => Poll::Ready(None),
    }
  }
}
