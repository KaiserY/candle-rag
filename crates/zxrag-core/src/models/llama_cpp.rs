use candle_core::quantized::{ggml_file, gguf_file};
use candle_core::utils::cuda_is_available;
use candle_core::{Device, Tensor};
use candle_examples::token_output_stream::TokenOutputStream;
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::quantized_llama as model;
use candle_transformers::models::quantized_llama::ModelWeights;
use futures::Stream;
use std::io::Write;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::OnceLock;
use std::task::{Context, Poll};
use tokenizers::Tokenizer;

use crate::types::conf::ChatCompletionSetting;
use crate::types::model::ModelId;
use crate::types::stopping_stream::StoppingStream;
use crate::util::format_size;

const DEFAULT_PROMPT: &str = "My favorite theorem is ";

pub static MODEL: OnceLock<Model> = OnceLock::new();

#[derive(Debug, Clone)]
enum Prompt {
  Interactive,
  Chat,
  One(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
  pub model_id: ModelId,
  pub model_path: String,
  pub tokenizer_path: String,
  pub device: String,
}

#[derive(Debug, Clone)]
pub struct Model {
  pub model_id: ModelId,
  pub model_path: String,
  pub tokenizer_path: String,
  pub device: Device,
  model_weights: ModelWeights,
  tokenizer: Tokenizer,
}

impl Model {
  pub fn new(cfg: &Config) -> anyhow::Result<Self> {
    tracing::info!(
      "avx: {}, neon: {}, simd128: {}, f16c: {}",
      candle_core::utils::with_avx(),
      candle_core::utils::with_neon(),
      candle_core::utils::with_simd128(),
      candle_core::utils::with_f16c()
    );

    let device = match cfg.device.as_str() {
      "cpu" => Device::Cpu,
      "cuda" => {
        if cuda_is_available() {
          Device::new_cuda(0)?
        } else {
          Device::Cpu
        }
      }
      _ => Device::Cpu,
    };

    let model_path = PathBuf::from(&cfg.model_path);
    let mut model_file = std::fs::File::open(&cfg.model_path)?;
    let start = std::time::Instant::now();

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

        let default_gqa = match &cfg.model_id {
          ModelId::Zephyr7bAlpha | ModelId::Zephyr7bBeta => 8,
          _ => 1,
        };
        ModelWeights::from_ggml(model, default_gqa)?
      }
    };

    tracing::info!("model built");

    let tokenizer =
      Tokenizer::from_file(PathBuf::from(&cfg.tokenizer_path)).map_err(anyhow::Error::msg)?;

    Ok(Self {
      model_id: cfg.model_id,
      model_path: cfg.model_path.clone(),
      tokenizer_path: cfg.tokenizer_path.clone(),
      device,
      model_weights,
      tokenizer,
    })
  }
}

pub struct TextGeneration {
  pub model: Model,
  token_output_stream: TokenOutputStream,
  logits_processor: LogitsProcessor,
  prompt: Prompt,
  sample_len: usize,
  repeat_penalty: f32,
  repeat_last_n: usize,
  all_tokens: Vec<u32>,
  eos_token: u32,
}

impl TextGeneration {
  pub fn new(model: Model, setting: ChatCompletionSetting) -> anyhow::Result<Self> {
    let temperature = if setting.temperature == 0. {
      None
    } else {
      Some(setting.temperature)
    };

    let token_output_stream = TokenOutputStream::new(model.tokenizer.clone());

    let logits_processor = LogitsProcessor::new(setting.seed, temperature, setting.top_p);

    let prompt = match setting.prompt.as_deref() {
      Some("chat") => Prompt::Chat,
      Some("interactive") => Prompt::Interactive,
      Some(s) => Prompt::One(s.to_string()),
      None => Prompt::One(DEFAULT_PROMPT.to_string()),
    };

    let eos_token = if model.model_id.is_open_chat() {
      "<|end_of_turn|>"
    } else {
      "</s>"
    };

    let eos_token = *token_output_stream
      .tokenizer()
      .get_vocab(true)
      .get(eos_token)
      .unwrap();

    Ok(Self {
      model,
      token_output_stream,
      logits_processor,
      prompt,
      sample_len: setting.sample_len,
      repeat_penalty: setting.repeat_penalty,
      repeat_last_n: setting.repeat_last_n,
      all_tokens: vec![],
      eos_token,
    })
  }

  pub fn run(&mut self) -> anyhow::Result<()> {
    let mut pre_prompt_tokens = vec![];

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
          if self.model.model_id.is_open_chat() {
            format!("GPT4 Correct User: {prompt}<|end_of_turn|>GPT4 Correct Assistant:")
          } else if self.model.model_id.is_zephyr() {
            if prompt_index == 0 || is_interactive {
              format!("<|system|>\n</s>\n<|user|>\n{prompt}</s>\n<|assistant|>",)
            } else {
              format!("<|user|>\n{prompt}</s>\n<|assistant|>")
            }
          } else if self.model.model_id.is_mistral() {
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

      let mut prompt_tokens = [&pre_prompt_tokens, tokens.get_ids()].concat();

      prompt_tokens = if prompt_tokens.len() + self.sample_len > model::MAX_SEQ_LEN - 10 {
        let to_remove = prompt_tokens.len() + self.sample_len + 10 - model::MAX_SEQ_LEN;
        prompt_tokens[prompt_tokens.len().saturating_sub(to_remove)..].to_vec()
      } else {
        prompt_tokens
      };

      self.all_tokens.extend(prompt_tokens.clone());

      let start_post_prompt = std::time::Instant::now();
      let mut sampled = 0;
      for index in 0..self.sample_len {
        let context_size = if index > 0 { 1 } else { self.all_tokens.len() };

        let start_pos = self.all_tokens.len().saturating_sub(context_size);

        let next_token = self.forward_token(start_pos)?;

        self.all_tokens.push(next_token);

        if let Some(t) = self.token_output_stream.next_token(next_token)? {
          tracing::info!("t={}", t);

          print!("{t}");
          std::io::stdout().flush()?;
        }

        sampled += 1;

        if next_token == self.eos_token {
          break;
        };
      }
      if let Some(rest) = self
        .token_output_stream
        .decode_rest()
        .map_err(candle_core::Error::msg)?
      {
        tracing::info!("rest={}", rest);

        print!("{rest}");
      }

      std::io::stdout().flush()?;
      let dt = start_post_prompt.elapsed();

      tracing::info!(
        "\n\n{sampled:4} tokens generated: {:.2} token/s",
        sampled as f64 / dt.as_secs_f64(),
      );

      match self.prompt {
        Prompt::One(_) => break,
        Prompt::Interactive => {}
        Prompt::Chat => {
          pre_prompt_tokens = [prompt_tokens.as_slice(), self.all_tokens.as_slice()].concat()
        }
      }
    }

    Ok(())
  }

  pub fn generate(&mut self) -> anyhow::Result<String> {
    let prompt_str = match &self.prompt {
      Prompt::One(prompt) => prompt.clone(),
      _ => "".to_string(),
    };

    tracing::info!("prompt_str={}", &prompt_str);

    let tokens = self
      .token_output_stream
      .tokenizer()
      .encode(prompt_str, true)
      .map_err(anyhow::Error::msg)?;

    let mut prompt_tokens = tokens.get_ids().to_owned();

    prompt_tokens = if prompt_tokens.len() + self.sample_len > model::MAX_SEQ_LEN - 10 {
      let to_remove = prompt_tokens.len() + self.sample_len + 10 - model::MAX_SEQ_LEN;
      prompt_tokens[prompt_tokens.len().saturating_sub(to_remove)..].to_vec()
    } else {
      prompt_tokens
    };

    self.all_tokens.extend(prompt_tokens.clone());

    let mut output = String::new();

    for index in 0..self.sample_len {
      let context_size = if index > 0 { 1 } else { self.all_tokens.len() };

      let start_pos = self.all_tokens.len().saturating_sub(context_size);

      let next_token = self.forward_token(start_pos)?;

      self.all_tokens.push(next_token);

      if let Some(t) = self.token_output_stream.next_token(next_token)? {
        tracing::info!("t={}", t);

        output.push_str(&t);
      }

      if next_token == self.eos_token {
        break;
      };
    }
    if let Some(rest) = self
      .token_output_stream
      .decode_rest()
      .map_err(candle_core::Error::msg)?
    {
      tracing::info!("rest={}", rest);

      output.push_str(&rest);
    }

    Ok(output)
  }

  fn forward_token(&mut self, index_pos: usize) -> anyhow::Result<u32> {
    let ctxt = &self.all_tokens[index_pos..];

    let input = Tensor::new(ctxt, &self.model.device)?.unsqueeze(0)?;

    let logits = self.model.model_weights.forward(&input, index_pos)?;

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

pub struct TextGenerationStream {
  pub text_gen: TextGeneration,
  sampled: usize,
}

impl TextGenerationStream {
  pub fn new(mut text_gen: TextGeneration) -> anyhow::Result<Self> {
    let prompt_str = match &text_gen.prompt {
      Prompt::One(prompt) => prompt.clone(),
      _ => "".to_string(),
    };

    tracing::info!("prompt_str={}", &prompt_str);

    let tokens = text_gen
      .token_output_stream
      .tokenizer()
      .encode(prompt_str, true)
      .map_err(anyhow::Error::msg)?;

    let mut prompt_tokens = tokens.get_ids().to_owned();

    prompt_tokens = if prompt_tokens.len() + text_gen.sample_len > model::MAX_SEQ_LEN - 10 {
      let to_remove = prompt_tokens.len() + text_gen.sample_len + 10 - model::MAX_SEQ_LEN;
      prompt_tokens[prompt_tokens.len().saturating_sub(to_remove)..].to_vec()
    } else {
      prompt_tokens
    };

    text_gen.all_tokens.extend(prompt_tokens.clone());

    Ok(Self {
      text_gen,
      sampled: 0,
    })
  }
}

impl Stream for TextGenerationStream {
  type Item = String;

  fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    tracing::info!("sampled={}", self.sampled);

    if self.sampled > self.text_gen.sample_len {
      return Poll::Ready(None);
    }

    let context_size = if self.sampled > 0 {
      1
    } else {
      self.text_gen.all_tokens.len()
    };

    let start_pos = self.text_gen.all_tokens.len().saturating_sub(context_size);

    let next_token = self.text_gen.forward_token(start_pos).ok();

    match next_token {
      Some(next_token) => {
        self.text_gen.all_tokens.push(next_token);
        self.sampled += 1;

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
      None => Poll::Ready(None),
    }
  }
}

pub async fn chat_completion(mut text_gen: TextGeneration) -> anyhow::Result<String> {
  text_gen.generate()
}

pub async fn chat_completion_stream(
  stream: TextGenerationStream,
) -> anyhow::Result<StoppingStream<Box<dyn Stream<Item = String> + Unpin + Send>>> {
  let pinned = Box::pin(Box::new(stream));

  Ok(StoppingStream::wrap_with_stop_words(
    Box::new(pinned),
    vec![
      "<|ASSISTANT|>".to_string(),
      "<|USER|>".to_string(),
      "<|TOOL|>".to_string(),
      "<|SYSTEM|>".to_string(),
    ],
  ))
}
