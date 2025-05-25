use std::{
    cell::Cell,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Wake},
};

use crossbeam::sync::{Parker, Unparker};

#[must_use = "checkpoints do nothing unless you `.await` or poll them"]
pub struct Checkpoint;

impl Future for Checkpoint {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if AT_CHECKPOINT.get() {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

pub fn checkpoint() -> Checkpoint {
    AT_CHECKPOINT.set(true);
    Checkpoint
}

struct ThreadWaker(Unparker);

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

pub fn until_checkpoint<T>(mut fut: Pin<&mut (impl Future<Output = T> + ?Sized)>) -> Option<T> {
    // Use a `Parker` instance rather than global `thread::park/unpark`, so no one
    // else can steal our `unpark`s and they don't get confused with recursive
    // `block_on` `unpark`s.
    let parker = Parker::new();
    // Make sure we create a new waker each call, rather than using a global, so
    // recursive `block_on`s don't use the same waker.
    let waker = Arc::new(ThreadWaker(parker.unparker().clone())).into();
    let mut cx = Context::from_waker(&waker);

    // Run the future until we're at a checkpoint.
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(res) => return Some(res),
            Poll::Pending => {
                if AT_CHECKPOINT.replace(false) {
                    return None;
                }

                parker.park()
            }
        }
    }
}

thread_local! {
    static AT_CHECKPOINT: Cell<bool> = const { Cell::new(false) };
}
