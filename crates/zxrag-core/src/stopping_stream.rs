use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
#[allow(unused_imports)]
use futures::StreamExt;

#[pin_project::pin_project]
pub struct StoppingStream<T> {
  #[pin]
  inner: T,

  stop_words: Vec<String>,

  working_buf: String,

  is_fused: bool,
}

impl<T> StoppingStream<T>
where
  T: Stream<Item = String>,
{
  pub fn wrap_with_stop_words(inner: T, stop_words: impl Into<Vec<String>>) -> Self {
    Self {
      inner,
      stop_words: stop_words.into(),
      working_buf: String::new(),
      is_fused: false,
    }
  }
}

impl<T> Stream for StoppingStream<T>
where
  T: Stream<Item = String> + Unpin,
{
  type Item = String;

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut this = self.as_mut().project();

    if *this.is_fused {
      return Poll::Ready(None);
    }

    loop {
      let token = match this.inner.as_mut().poll_next(cx) {
        Poll::Ready(Some(token)) => token,
        Poll::Ready(None) => {
          self.is_fused = true;

          return Poll::Ready(None);
        }
        Poll::Pending => return Poll::Pending,
      };

      this.working_buf.push_str(&token);

      let mut should_emit = true;

      'stop_words: for stop_word in &*this.stop_words {
        if this.working_buf.starts_with(stop_word) {
          return Poll::Ready(None);
        }

        if stop_word.starts_with(&*this.working_buf) {
          should_emit = false;

          break 'stop_words;
        }
      }

      if should_emit {
        let out_buf = std::mem::take(this.working_buf);

        return Poll::Ready(Some(out_buf));
      }
    }
  }
}
