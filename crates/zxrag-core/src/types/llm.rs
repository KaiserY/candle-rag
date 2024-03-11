use candle_core::{Device, Tensor};
use candle_transformers::generation::LogitsProcessor;
use dyn_clone::DynClone;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokenizers::Tokenizer;

use crate::types::model::{ModelEngine, ModelId};

pub trait LlmModel: DynClone {
  fn id(&self) -> ModelId;
  fn engine(&self) -> ModelEngine;
  fn tokenizer(&self) -> &Tokenizer;
  fn device(&self) -> &Device;
  fn forward(&mut self, x: &Tensor, index_pos: usize) -> anyhow::Result<Tensor>;
}

dyn_clone::clone_trait_object!(LlmModel);

pub struct TextGenerationSetting {
  pub temperature: f64,
  pub top_p: Option<f64>,
  pub seed: u64,
  pub repeat_penalty: f32,
  pub repeat_last_n: usize,
  pub sample_len: usize,
  pub prompt: String,
}

pub struct TextGeneration<T: LlmModel> {
  pub model: T,
  pub setting: TextGenerationSetting,
  pub logits_processor: LogitsProcessor,
  all_tokens: Vec<u32>,
  eos_token: u32,
}

impl<T> TextGeneration<T>
where
  T: LlmModel,
{
  pub fn new(model: T, setting: TextGenerationSetting) -> anyhow::Result<Self> {
    let temperature = if setting.temperature == 0. {
      None
    } else {
      Some(setting.temperature)
    };

    let logits_processor = LogitsProcessor::new(setting.seed, temperature, setting.top_p);

    let eos_token = if model.id().is_open_chat() {
      "<|end_of_turn|>"
    } else {
      "</s>"
    };

    let eos_token = *model.tokenizer().get_vocab(true).get(eos_token).unwrap();

    Ok(Self {
      model,
      setting,
      logits_processor,
      all_tokens: vec![],
      eos_token,
    })
  }

  pub fn generate(&mut self) -> anyhow::Result<String> {
    tracing::info!("prompt={}", self.setting.prompt);

    let tokens = self
      .model
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

      let token_str = self
        .model
        .tokenizer()
        .decode(&[next_token], true)
        .map_err(anyhow::Error::msg)?;

      output.push_str(&token_str);
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

pub struct TextGenerationStream<T: LlmModel> {
  pub text_gen: TextGeneration<T>,
  generated_tokens: usize,
}

impl<T> Stream for TextGenerationStream<T>
where
  T: LlmModel + Unpin,
{
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
        } else if let Ok(t) = self.text_gen.model.tokenizer().decode(&[next_token], true) {
          tracing::info!("t={:?}", t);

          Poll::Ready(Some(t))
        } else {
          Poll::Ready(None)
        }
      }
      Err(_) => Poll::Ready(None),
    }
  }
}
