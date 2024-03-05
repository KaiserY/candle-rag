use std::pin::Pin;
use std::task::{Context, Poll};
use futures::Stream;
#[allow(unused_imports)]
use futures::StreamExt;

use crate::llama_cpp::LlamaCppModel;

pub struct CompletionStream {
  pub model: LlamaCppModel
}

impl CompletionStream {}

impl Stream for CompletionStream {
  type Item = String;

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
      match std::pin::pin!(&mut self.model).poll_next(cx) {
          Poll::Ready(Some(val)) => {
              if let Some(id) = &mut self.session_id {
                  id.advance(&val);
              }
              Poll::Ready(Some(val))
          }
          Poll::Ready(None) => Poll::Ready(None),
          Poll::Pending => Poll::Pending,
      }
  }
}
