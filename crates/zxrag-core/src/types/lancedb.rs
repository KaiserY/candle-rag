use arrow_schema::{DataType, Field, Schema};
use std::sync::Arc;
use std::sync::OnceLock;

pub static EMBEDDING_SCHEMA: OnceLock<Arc<Schema>> = OnceLock::new();

pub fn set_embedding_schema() -> anyhow::Result<()> {
  EMBEDDING_SCHEMA
    .set(Arc::new(Schema::new(vec![
      Field::new("id", DataType::Utf8, false),
      Field::new("kb_id", DataType::Int64, false),
      Field::new("file_id", DataType::Int64, false),
      Field::new("file_name", DataType::Utf8, false),
      Field::new("text", DataType::Utf8, false),
      Field::new(
        "vector",
        DataType::FixedSizeList(
          Arc::new(Field::new("item", DataType::Float32, true)),
          1024_i32,
        ),
        false,
      ),
    ])))
    .map_err(|_| anyhow::anyhow!("init_embedding_schema failed"))?;

  Ok(())
}

pub fn get_embedding_schema() -> anyhow::Result<Arc<Schema>> {
  let schema = EMBEDDING_SCHEMA
    .get()
    .ok_or(anyhow::anyhow!("get_embedding_schema failed"))?;

  Ok(schema.clone())
}
