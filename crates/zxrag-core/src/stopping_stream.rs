use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
#[allow(unused_imports)]
use futures::StreamExt;

#[pin_project::pin_project]
pub struct StoppingStream<T> {
  #[pin]
  inner: T,
}

impl<T> StoppingStream<T>
where
  T: Stream<Item = String>,
{
  pub fn new(inner: T) -> Self {
    Self { inner }
  }
}

impl<T> Stream for StoppingStream<T>
where
  T: Stream<Item = String> + Unpin,
{
  type Item = String;

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    match std::pin::pin!(&mut self.inner).poll_next(cx) {
      Poll::Ready(Some(val)) => Poll::Ready(Some(val)),
      Poll::Ready(None) => Poll::Ready(None),
      Poll::Pending => Poll::Pending,
    }
  }
}
