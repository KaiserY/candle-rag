use std::io::Write;
use std::path::PathBuf;
use tokenizers::Tokenizer;

use candle_core::quantized::{ggml_file, gguf_file};
use candle_core::Tensor;
use candle_transformers::generation::LogitsProcessor;

use candle_examples::token_output_stream::TokenOutputStream;
use candle_transformers::models::quantized_llama as model;
use model::ModelWeights;

use crate::conf::BackendConf;
use crate::model::{ModelChatSetting, ModelId};

const DEFAULT_PROMPT: &str = "hello ";

pub struct LlamaCppModel {

}

#[derive(Debug)]
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

pub fn run_quantized_llm(
  conf: &BackendConf,
  setting: &ModelChatSetting,
) -> Result<(), anyhow::Error> {
  let temperature = if setting.temperature == 0. {
    None
  } else {
    Some(setting.temperature)
  };

  println!(
    "avx: {}, neon: {}, simd128: {}, f16c: {}",
    candle_core::utils::with_avx(),
    candle_core::utils::with_neon(),
    candle_core::utils::with_simd128(),
    candle_core::utils::with_f16c()
  );
  println!(
    "temp: {:.2} repeat-penalty: {:.2} repeat-last-n: {}",
    setting.temperature, setting.repeat_penalty, setting.repeat_last_n
  );

  let model_path = PathBuf::from(&conf.model_path);
  let mut file = std::fs::File::open(&conf.model_path)?;
  let start = std::time::Instant::now();
  let device = candle_examples::device(true)?;

  let mut model = match model_path.extension().and_then(|v| v.to_str()) {
    Some("gguf") => {
      let model = gguf_file::Content::read(&mut file).map_err(|e| e.with_path(model_path))?;
      let mut total_size_in_bytes = 0;
      for (_, tensor) in model.tensor_infos.iter() {
        let elem_count = tensor.shape.elem_count();
        total_size_in_bytes +=
          elem_count * tensor.ggml_dtype.type_size() / tensor.ggml_dtype.block_size();
      }
      println!(
        "loaded {:?} tensors ({}) in {:.2}s",
        model.tensor_infos.len(),
        &format_size(total_size_in_bytes),
        start.elapsed().as_secs_f32(),
      );
      ModelWeights::from_gguf(model, &mut file, &device)?
    }
    Some("ggml" | "bin") | Some(_) | None => {
      let model =
        ggml_file::Content::read(&mut file, &device).map_err(|e| e.with_path(model_path))?;
      let mut total_size_in_bytes = 0;
      for (_, tensor) in model.tensors.iter() {
        let elem_count = tensor.shape().elem_count();
        total_size_in_bytes +=
          elem_count * tensor.dtype().type_size() / tensor.dtype().block_size();
      }
      println!(
        "loaded {:?} tensors ({}) in {:.2}s",
        model.tensors.len(),
        &format_size(total_size_in_bytes),
        start.elapsed().as_secs_f32(),
      );
      println!("params: {:?}", model.hparams);
      let default_gqa = match conf.model_id {
        ModelId::Zephyr7bAlpha | ModelId::Zephyr7bBeta => 8,
        _ => 1,
      };
      ModelWeights::from_ggml(model, default_gqa)?
    }
  };
  println!("model built");

  let tokenizer =
    Tokenizer::from_file(PathBuf::from(&conf.tokenizer_path)).map_err(anyhow::Error::msg)?;

  let mut tos = TokenOutputStream::new(tokenizer);
  let prompt = match setting.prompt.as_deref() {
    Some("chat") => Prompt::Chat,
    Some("interactive") => Prompt::Interactive,
    Some(s) => Prompt::One(s.to_string()),
    None => Prompt::One(DEFAULT_PROMPT.to_string()),
  };

  let mut pre_prompt_tokens = vec![];
  for prompt_index in 0.. {
    let prompt_str = match &prompt {
      Prompt::One(prompt) => prompt.clone(),
      Prompt::Interactive | Prompt::Chat => {
        let is_interactive = matches!(prompt, Prompt::Interactive);
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
        if conf.model_id.is_open_chat() {
          format!("GPT4 Correct User: {prompt}<|end_of_turn|>GPT4 Correct Assistant:")
        } else if conf.model_id.is_zephyr() {
          if prompt_index == 0 || is_interactive {
            format!("<|system|>\n</s>\n<|user|>\n{prompt}</s>\n<|assistant|>",)
          } else {
            format!("<|user|>\n{prompt}</s>\n<|assistant|>")
          }
        } else if conf.model_id.is_mistral() {
          format!("[INST] {prompt} [/INST]")
        } else {
          prompt
        }
      }
    };
    print!("{}", &prompt_str);
    let tokens = tos
      .tokenizer()
      .encode(prompt_str, true)
      .map_err(anyhow::Error::msg)?;

    let prompt_tokens = [&pre_prompt_tokens, tokens.get_ids()].concat();
    let to_sample = setting.sample_len.saturating_sub(1);
    let prompt_tokens = if prompt_tokens.len() + to_sample > model::MAX_SEQ_LEN - 10 {
      let to_remove = prompt_tokens.len() + to_sample + 10 - model::MAX_SEQ_LEN;
      prompt_tokens[prompt_tokens.len().saturating_sub(to_remove)..].to_vec()
    } else {
      prompt_tokens
    };
    let mut all_tokens = vec![];
    let mut logits_processor = LogitsProcessor::new(setting.seed, temperature, setting.top_p);

    let start_prompt_processing = std::time::Instant::now();
    let mut next_token = if !setting.split_prompt {
      let input = Tensor::new(prompt_tokens.as_slice(), &device)?.unsqueeze(0)?;
      let logits = model.forward(&input, 0)?;
      let logits = logits.squeeze(0)?;
      logits_processor.sample(&logits)?
    } else {
      let mut next_token = 0;
      for (pos, token) in prompt_tokens.iter().enumerate() {
        let input = Tensor::new(&[*token], &device)?.unsqueeze(0)?;
        let logits = model.forward(&input, pos)?;
        let logits = logits.squeeze(0)?;
        next_token = logits_processor.sample(&logits)?
      }
      next_token
    };
    let prompt_dt = start_prompt_processing.elapsed();
    all_tokens.push(next_token);
    if let Some(t) = tos.next_token(next_token)? {
      print!("{t}");
      std::io::stdout().flush()?;
    }

    let eos_token = if conf.model_id.is_open_chat() {
      "<|end_of_turn|>"
    } else {
      "</s>"
    };
    let eos_token = *tos.tokenizer().get_vocab(true).get(eos_token).unwrap();
    let start_post_prompt = std::time::Instant::now();
    let mut sampled = 0;
    for index in 0..to_sample {
      let input = Tensor::new(&[next_token], &device)?.unsqueeze(0)?;
      let logits = model.forward(&input, prompt_tokens.len() + index)?;
      let logits = logits.squeeze(0)?;
      let logits = if setting.repeat_penalty == 1. {
        logits
      } else {
        let start_at = all_tokens.len().saturating_sub(setting.repeat_last_n);
        candle_transformers::utils::apply_repeat_penalty(
          &logits,
          setting.repeat_penalty,
          &all_tokens[start_at..],
        )?
      };
      next_token = logits_processor.sample(&logits)?;
      all_tokens.push(next_token);
      if let Some(t) = tos.next_token(next_token)? {
        print!("{t}");
        std::io::stdout().flush()?;
      }
      sampled += 1;
      if next_token == eos_token {
        break;
      };
    }
    if let Some(rest) = tos.decode_rest().map_err(candle_core::Error::msg)? {
      print!("{rest}");
    }
    std::io::stdout().flush()?;
    let dt = start_post_prompt.elapsed();
    println!(
      "\n\n{:4} prompt tokens processed: {:.2} token/s",
      prompt_tokens.len(),
      prompt_tokens.len() as f64 / prompt_dt.as_secs_f64(),
    );
    println!(
      "{sampled:4} tokens generated: {:.2} token/s",
      sampled as f64 / dt.as_secs_f64(),
    );

    match prompt {
      Prompt::One(_) => break,
      Prompt::Interactive => {}
      Prompt::Chat => {
        pre_prompt_tokens = [prompt_tokens.as_slice(), all_tokens.as_slice()].concat()
      }
    }
  }

  Ok(())
}
