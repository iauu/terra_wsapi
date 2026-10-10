use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tokio::sync::Notify;


#[derive(Clone, Debug)]
pub struct RequestGroup<T> {
    has_requested: Arc<Mutex<bool>>,
    requests: Arc<Mutex<Vec<oneshot::Sender<T>>>>,
    notifier: Arc<Notify>
}


impl<T> RequestGroup<T> {
    pub fn new() -> Self {
        Self {
            has_requested: Arc::new(Mutex::new(false)),
            requests: Arc::new(Mutex::new(vec![])),
            notifier: Arc::new(Notify::new())
        }
    }

    pub async fn push_request(&mut self, sender: oneshot::Sender<T>) {
        self.requests.lock().await.push(sender);
        if !self.has_requested.lock().await.deref() {
            self.notifier.notify_waiters();
        }
    }

    pub async fn wait_for(&self) -> () {
        self.notifier.notified().await;
    }

    pub async fn reset_lock(&mut self) -> () {
        *self.has_requested.lock().await.deref_mut() = false;
    }
}