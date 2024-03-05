use candle_core::quantized::{ggml_file, gguf_file};
use candle_core::{Device, Tensor};
use candle_examples::token_output_stream::TokenOutputStream;
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::quantized_llama as model;
use futures::Stream;
#[allow(unused_imports)]
use futures::StreamExt;
use model::ModelWeights;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::OnceLock;
use std::task::{Context, Poll};
use tokenizers::Tokenizer;

use crate::model::{ChatCompletionSetting, ModelId};
use crate::stopping_stream::StoppingStream;

const DEFAULT_PROMPT: &str = "hello ";

pub static LLAMA_CPP_MODEL: OnceLock<LlamaCppModel> = OnceLock::new();

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LlamaCppModelConf {
  pub model_id: ModelId,
  pub model_path: String,
  pub tokenizer_path: String,
}

#[derive(Debug, Clone)]
pub struct LlamaCppModel {
  pub config: LlamaCppModelConf,
  model_weights: ModelWeights,
  tokenizer: Tokenizer,
}

impl LlamaCppModel {
  pub fn load_model(config: LlamaCppModelConf) -> anyhow::Result<LlamaCppModel> {
    tracing::info!(
      "avx: {}, neon: {}, simd128: {}, f16c: {}",
      candle_core::utils::with_avx(),
      candle_core::utils::with_neon(),
      candle_core::utils::with_simd128(),
      candle_core::utils::with_f16c()
    );

    let model_path = PathBuf::from(&config.model_path);
    let mut model_file = std::fs::File::open(&config.model_path)?;
    let start = std::time::Instant::now();
    let device = candle_examples::device(true)?;

    let model_weights = match model_path.extension().and_then(|v| v.to_str()) {
      Some("gguf") => {
        let model =
          gguf_file::Content::read(&mut model_file).map_err(|e| e.with_path(model_path))?;
        let mut total_size_in_bytes = 0;
        for (_, tensor) in model.tensor_infos.iter() {
          let elem_count = tensor.shape.elem_count();
          total_size_in_bytes +=
            elem_count * tensor.ggml_dtype.type_size() / tensor.ggml_dtype.block_size();
        }

        tracing::info!(
          "loaded {:?} tensors ({}) in {:.2}s",
          model.tensor_infos.len(),
          &format_size(total_size_in_bytes),
          start.elapsed().as_secs_f32(),
        );

        ModelWeights::from_gguf(model, &mut model_file, &device)?
      }
      Some("ggml" | "bin") | Some(_) | None => {
        let model = ggml_file::Content::read(&mut model_file, &device)
          .map_err(|e| e.with_path(model_path))?;
        let mut total_size_in_bytes = 0;
        for (_, tensor) in model.tensors.iter() {
          let elem_count = tensor.shape().elem_count();
          total_size_in_bytes +=
            elem_count * tensor.dtype().type_size() / tensor.dtype().block_size();
        }

        tracing::info!(
          "loaded {:?} tensors ({}) in {:.2}s",
          model.tensors.len(),
          &format_size(total_size_in_bytes),
          start.elapsed().as_secs_f32(),
        );

        tracing::info!("params: {:?}", model.hparams);

        let default_gqa = match &config.model_id {
          ModelId::Zephyr7bAlpha | ModelId::Zephyr7bBeta => 8,
          _ => 1,
        };
        ModelWeights::from_ggml(model, default_gqa)?
      }
    };

    tracing::info!("model built");

    let tokenizer =
      Tokenizer::from_file(PathBuf::from(&config.tokenizer_path)).map_err(anyhow::Error::msg)?;

    Ok(LlamaCppModel {
      config,
      model_weights,
      tokenizer,
    })
  }
}

pub struct LlamaCppModelPipeline {
  pub model: LlamaCppModel,
  prompt: Prompt,
  device: Device,
  token_output_stream: TokenOutputStream,
  pre_prompt_tokens: Vec<u32>,
  to_sample: usize,
  prompt_tokens: Vec<u32>,
  all_tokens: Vec<u32>,
  next_token: u32,
  eos_token: u32,
  logits_processor: LogitsProcessor,
  repeat_penalty: f32,
  repeat_last_n: usize,
}

impl LlamaCppModelPipeline {
  pub fn init_pipeline(
    model: LlamaCppModel,
    setting: ChatCompletionSetting,
  ) -> anyhow::Result<Self> {
    let temperature = if setting.temperature == 0. {
      None
    } else {
      Some(setting.temperature)
    };

    let device = candle_examples::device(true)?;

    let prompt = match setting.prompt.as_deref() {
      Some("chat") => Prompt::Chat,
      Some("interactive") => Prompt::Interactive,
      Some(s) => Prompt::One(s.to_string()),
      None => Prompt::One(DEFAULT_PROMPT.to_string()),
    };

    let token_output_stream = TokenOutputStream::new(model.tokenizer.clone());

    let logits_processor = LogitsProcessor::new(setting.seed, temperature, setting.top_p);

    let eos_token = if model.config.model_id.is_open_chat() {
      "<|end_of_turn|>"
    } else {
      "</s>"
    };

    let eos_token = *token_output_stream
      .tokenizer()
      .get_vocab(true)
      .get(eos_token)
      .unwrap();

    Ok(LlamaCppModelPipeline {
      model,
      device,
      prompt,
      token_output_stream,
      pre_prompt_tokens: vec![],
      to_sample: setting.sample_len.saturating_sub(1),
      prompt_tokens: vec![],
      all_tokens: vec![],
      next_token: 0,
      eos_token: eos_token,
      logits_processor,
      repeat_penalty: setting.repeat_penalty,
      repeat_last_n: setting.repeat_last_n,
    })
  }

  pub fn run_cli(&mut self) -> anyhow::Result<()> {
    for prompt_index in 0.. {
      let prompt_str = match &self.prompt {
        Prompt::One(prompt) => prompt.clone(),
        Prompt::Interactive | Prompt::Chat => {
          let is_interactive = matches!(self.prompt, Prompt::Interactive);
          print!("> ");
          std::io::stdout().flush()?;
          let mut prompt = String::new();
          std::io::stdin().read_line(&mut prompt)?;
          if prompt.ends_with('\n') {
            prompt.pop();
            if prompt.ends_with('\r') {
              prompt.pop();
            }
          }
          if self.model.config.model_id.is_open_chat() {
            format!("GPT4 Correct User: {prompt}<|end_of_turn|>GPT4 Correct Assistant:")
          } else if self.model.config.model_id.is_zephyr() {
            if prompt_index == 0 || is_interactive {
              format!("<|system|>\n</s>\n<|user|>\n{prompt}</s>\n<|assistant|>",)
            } else {
              format!("<|user|>\n{prompt}</s>\n<|assistant|>")
            }
          } else if self.model.config.model_id.is_mistral() {
            format!("[INST] {prompt} [/INST]")
          } else {
            prompt
          }
        }
      };

      print!("{}", &prompt_str);

      let tokens = self
        .token_output_stream
        .tokenizer()
        .encode(prompt_str, true)
        .map_err(anyhow::Error::msg)?;

      let prompt_tokens = [&self.pre_prompt_tokens, tokens.get_ids()].concat();

      self.prompt_tokens = if prompt_tokens.len() + self.to_sample > model::MAX_SEQ_LEN - 10 {
        let to_remove = prompt_tokens.len() + self.to_sample + 10 - model::MAX_SEQ_LEN;
        prompt_tokens[prompt_tokens.len().saturating_sub(to_remove)..].to_vec()
      } else {
        prompt_tokens
      };

      let start_prompt_processing = std::time::Instant::now();

      let input = Tensor::new(self.prompt_tokens.as_slice(), &self.device)?.unsqueeze(0)?;
      let logits = self.model.model_weights.forward(&input, 0)?;
      let logits = logits.squeeze(0)?;
      self.next_token = self.logits_processor.sample(&logits)?;

      let prompt_dt = start_prompt_processing.elapsed();

      self.all_tokens.push(self.next_token);

      if let Some(t) = self.token_output_stream.next_token(self.next_token)? {
        tracing::info!("t={}", t);

        print!("{t}");
        std::io::stdout().flush()?;
      }

      let start_post_prompt = std::time::Instant::now();
      let mut sampled = 0;
      for index in 0..self.to_sample {
        self.next_token = self.forward(index)?;

        self.all_tokens.push(self.next_token);

        if let Some(t) = self.token_output_stream.next_token(self.next_token)? {
          print!("{t}");
          std::io::stdout().flush()?;
        }
        sampled += 1;
        if self.next_token == self.eos_token {
          break;
        };
      }
      if let Some(rest) = self
        .token_output_stream
        .decode_rest()
        .map_err(candle_core::Error::msg)?
      {
        print!("{rest}");
      }

      std::io::stdout().flush()?;
      let dt = start_post_prompt.elapsed();

      tracing::info!(
        "\n\n{:4} prompt tokens processed: {:.2} token/s",
        self.prompt_tokens.len(),
        self.prompt_tokens.len() as f64 / prompt_dt.as_secs_f64(),
      );

      tracing::info!(
        "{sampled:4} tokens generated: {:.2} token/s",
        sampled as f64 / dt.as_secs_f64(),
      );

      match self.prompt {
        Prompt::One(_) => break,
        Prompt::Interactive => {}
        Prompt::Chat => {
          self.pre_prompt_tokens =
            [self.prompt_tokens.as_slice(), self.all_tokens.as_slice()].concat()
        }
      }
    }

    Ok(())
  }

  fn forward(&mut self, index: usize) -> anyhow::Result<u32> {
    let input = Tensor::new(&[self.next_token], &self.device)?.unsqueeze(0)?;

    let logits = self
      .model
      .model_weights
      .forward(&input, self.prompt_tokens.len() + index)?;

    let logits = logits.squeeze(0)?;

    let logits = if self.repeat_penalty == 1. {
      logits
    } else {
      let start_at = self.all_tokens.len().saturating_sub(self.repeat_last_n);
      candle_transformers::utils::apply_repeat_penalty(
        &logits,
        self.repeat_penalty,
        &self.all_tokens[start_at..],
      )?
    };

    Ok(self.logits_processor.sample(&logits)?)
  }
}

#[derive(Debug, Clone)]
enum Prompt {
  Interactive,
  Chat,
  One(String),
}

fn format_size(size_in_bytes: usize) -> String {
  if size_in_bytes < 1_000 {
    format!("{}B", size_in_bytes)
  } else if size_in_bytes < 1_000_000 {
    format!("{:.2}KB", size_in_bytes as f64 / 1e3)
  } else if size_in_bytes < 1_000_000_000 {
    format!("{:.2}MB", size_in_bytes as f64 / 1e6)
  } else {
    format!("{:.2}GB", size_in_bytes as f64 / 1e9)
  }
}

#[pin_project::pin_project]
pub struct LlamaCppChatCompletionStream {
  pub model: LlamaCppModel,
  pub setting: ChatCompletionSetting,
}

impl Stream for LlamaCppChatCompletionStream {
  type Item = String;

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    match std::pin::pin!(&mut self).poll_next(cx) {
      Poll::Ready(Some(val)) => Poll::Ready(Some(val)),
      Poll::Ready(None) => Poll::Ready(None),
      Poll::Pending => Poll::Pending,
    }
  }
}

impl Iterator for LlamaCppChatCompletionStream {
  type Item = String;

  fn next(&mut self) -> Option<Self::Item> {
    Some("".to_string())
  }
}

pub async fn chat_completion_stream(
  stream: LlamaCppChatCompletionStream,
) -> Result<StoppingStream<Box<dyn Stream<Item = String> + Unpin + Send>>, anyhow::Error> {
  Ok(StoppingStream::new(Box::new(Box::pin(Box::new(stream)))))
}
