use arrow_array::{
  types::Float32Type, FixedSizeListArray, Int32Array, RecordBatch, RecordBatchIterator, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use clap::Args;
use clap::{Parser, Subcommand};
use futures::TryStreamExt;
use std::sync::Arc;
use time::format_description::well_known;
use time::UtcOffset;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use zxrag_backend::run_backend;
use zxrag_core::types::conf::{init_backend_conf, BackendConf, LlmConf};
use zxrag_core::types::handle::{
  get_embedding_model, get_text_gen, set_embedding_model_handle, set_llm_model_handle,
};
use zxrag_core::types::lancedb::set_embedding_schema;
use zxrag_core::types::llm::TextGenerationSetting;
use zxrag_core::types::model::{ModelEngine, ModelId};

#[derive(Debug, Default, Args)]
pub struct CliConfig {
  #[clap(long, default_value_t = ModelId::None)]
  pub model_id: ModelId,
  #[clap(long, default_value_t = ModelEngine::Gguf)]
  pub model_engine: ModelEngine,
  #[clap(long)]
  pub model_path: String,
  #[clap(long)]
  pub repo_id: String,
  #[clap(long)]
  pub tokenizer_path: String,
  #[clap(long, default_value_t = String::from("cpu"))]
  pub device: String,
  #[arg(long, default_value_t = 1.1)]
  repeat_penalty: f32,
  #[arg(long, default_value_t = 64)]
  repeat_last_n: usize,
  #[arg(long, default_value_t = String::from("Hello !"))]
  prompt: String,
  #[arg(long, default_value_t = 0.8)]
  temperature: f64,
  #[arg(long)]
  top_p: Option<f64>,
  #[arg(long, default_value_t = 299792458)]
  seed: u64,
  #[arg(long, short = 'n', default_value_t = 10000)]
  sample_len: usize,
}

#[derive(Debug, Default, Args)]
pub struct BackendConfig {
  #[clap(long, default_value_t = String::from("zxrag.toml"))]
  pub config: String,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  #[clap(about = "Run the backend")]
  Backend(BackendConfig),
  #[clap(about = "Run the Cli")]
  Cli(CliConfig),
  #[clap(about = "Run the Test")]
  Test(BackendConfig),
}

fn main() -> Result<(), anyhow::Error> {
  let cli = Cli::parse();

  match cli.command {
    Commands::Cli(cli_config) => {
      let llm_conf = LlmConf{
        enabled: true,
        model_id: cli_config.model_id,
        model_engine: cli_config.model_engine,
        model_path :cli_config.model_path,
        repo_id: cli_config.repo_id,
        tokenizer_path: cli_config.tokenizer_path,
        device: cli_config.device,
      };

      let text_gen_setting = TextGenerationSetting{
        temperature: cli_config.temperature,
        top_p: cli_config.top_p,
        seed: cli_config.seed,
        repeat_penalty: cli_config.repeat_penalty,
        repeat_last_n: cli_config.repeat_last_n,
        sample_len: cli_config.sample_len,
        prompt: cli_config.prompt,
      };


    }
    Commands::Backend(cli_config) => {
      let config: BackendConf = init_backend_conf(&cli_config.config)?;

      tracing::info!("config={:?}", config);

      let offset = UtcOffset::from_hms(8, 0, 0).expect("should get CST offset");

      let timer = tracing_subscriber::fmt::time::OffsetTime::new(offset, well_known::Rfc3339);

      let file_appender =
        tracing_appender::rolling::daily(&config.log_file_path, &config.log_file_name);
      let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

      tracing_subscriber::registry()
        .with(
          tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(
          tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_timer(timer.clone())
            .with_ansi(false),
        )
        .with(
          tracing_subscriber::fmt::layer()
            .with_writer(std::io::stdout)
            .with_timer(timer),
        )
        .init();

      set_llm_model_handle(config.llm_conf.model_id, &config.llm_conf)?;

      set_embedding_model_handle(config.embedding_conf.model_id, &config.embedding_conf)?;

      set_embedding_schema()?;

      run_backend(config)?;
    }
    Commands::Test(cli_config) => {
      let config: BackendConf = init_backend_conf(&cli_config.config)?;

      tracing::info!("config={:?}", config);

      tracing_subscriber::fmt()
        .with_level(false)
        .with_ansi(false)
        .with_target(false)
        .without_time()
        .init();

      set_llm_model_handle(config.llm_conf.model_id, &config.llm_conf)?;

      let text_gen_setting = TextGenerationSetting {
        temperature: 0.8,
        top_p: None,
        seed: 299792458,
        repeat_penalty: 1.1,
        repeat_last_n: 64,
        sample_len: 16,
        // prompt: "<s>[INST] Hello! [/INST]".to_string(),
        // prompt: "<|user|>\nHello!</s>\n<|assistant|>".to_string(),
        prompt: "Alice: Hello!\nBob: ".to_string(),
      };

      let mut text_gen = get_text_gen(text_gen_setting)?;

      let output = text_gen.generate()?;

      tracing::info!("{output}");

      set_embedding_model_handle(config.embedding_conf.model_id, &config.embedding_conf)?;

      let bert_model = get_embedding_model(config.embedding_conf.model_id)?;

      let sentences = [
        "The cat sits outside",
        "A man is playing guitar",
        "I love pasta",
        "The new movie is awesome",
        "The cat plays in the garden",
        "A woman watches TV",
        "The new movie is so great",
        "Do you like pizza?",
      ];

      const DIM: usize = 1024;

      let runtime = tokio::runtime::Runtime::new()?;

      runtime.block_on(async {
        let db = vectordb::connect(&config.lancedb_path).await?;

        let tables = db.table_names().await?;

        let table_name = "my_table".to_string();

        if tables.contains(&table_name) {
          db.drop_table(&table_name).await?;
        }

        let schema = Arc::new(Schema::new(vec![
          Field::new("id", DataType::Int32, false),
          Field::new("text", DataType::Utf8, true),
          Field::new(
            "vector",
            DataType::FixedSizeList(
              Arc::new(Field::new("item", DataType::Float32, true)),
              DIM as i32,
            ),
            true,
          ),
        ]));

        let batches = RecordBatchIterator::new(vec![], schema.clone());

        let tbl = db.create_table("my_table", Box::new(batches), None).await?;

        let embeddings: Vec<Vec<f32>> = bert_model.embedding_batch(&sentences)?;

        let query = embeddings[2].clone();

        let vectors: Vec<Option<Vec<Option<f32>>>> = embeddings
          .into_iter()
          .map(|t| Some(t.into_iter().map(Some).collect()))
          .collect();

        for item in &vectors {
          tracing::info!("{}", item.as_ref().unwrap().len())
        }

        let batches = RecordBatchIterator::new(
          vec![RecordBatch::try_new(
            schema.clone(),
            vec![
              Arc::new(Int32Array::from_iter_values(0..sentences.len() as i32)),
              Arc::new(StringArray::from(
                sentences
                  .into_iter()
                  .map(Some)
                  .collect::<Vec<Option<&str>>>(),
              )),
              Arc::new(
                FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(vectors, DIM as i32),
              ),
            ],
          )?]
          .into_iter()
          .map(Ok),
          schema.clone(),
        );

        tbl.add(Box::new(batches), None).await?;

        let result = tbl
          .search(&query)
          // .query().filter("id = 3")
          .limit(2)
          .execute_stream()
          .await?
          .try_collect::<Vec<_>>()
          .await?;

        tracing::info!("{:?}", result);

        Ok::<(), anyhow::Error>(())
      })?;
    }
  }

  Ok(())
}
