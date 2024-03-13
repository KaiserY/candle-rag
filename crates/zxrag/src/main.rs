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
use vectordb::connect;
use zxrag_backend::run_backend;
use zxrag_core::types::conf::{init_backend_conf, BackendConf};
use zxrag_core::types::handle::{
  get_embedding_model, get_text_gen, set_embedding_model_handle, set_llm_model_handle,
};
use zxrag_core::types::llm::TextGenerationSetting;

#[derive(Debug, Default, Args)]
pub struct CliConfig {
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
  Backend(CliConfig),
  #[clap(about = "Run the Cli")]
  Cli(CliConfig),
}

fn main() -> Result<(), anyhow::Error> {
  let cli = Cli::parse();

  match cli.command {
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

      run_backend(config)?;
    }
    Commands::Cli(cli_config) => {
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

      let tenser = bert_model.embedding_batch(&["A man is playing guitar"])?;

      tracing::info!("{tenser}");

      let tenser = tenser.flatten_all()?;

      tracing::info!("{tenser}");

      let vector: Vec<Option<f32>> = tenser.to_vec1()?.into_iter().map(Some).collect();

      tracing::info!("{}", vector.len());

      let runtime = tokio::runtime::Runtime::new()?;

      runtime.block_on(async {
        let db = connect(&config.lancedb_path).await?;

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

        let embeddings: Vec<Vec<f32>> = bert_model.embedding_batch(&sentences)?.to_vec2()?;

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
