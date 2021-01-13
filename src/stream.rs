use tokio::stream::Stream;
use futures::task::Context;
use tokio::macros::support::{Pin, Poll};
use mongodb::Cursor;

struct Streaming {
    cursor: Cursor,
}

impl Stream for Streaming {
    type Item = ();

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        unimplemented!()
    }
}