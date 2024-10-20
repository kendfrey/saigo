use std::sync::{Arc, Mutex};

use tokio::sync::watch::{Receiver, Sender};

/// Allows a caller to take sole ownership of a Sender.
/// Resets the value to the default value when ownership is released.
pub(crate) struct SenderLock<T> {
    sender: Sender<T>,
    is_owned: Mutex<bool>,
}

impl<T: Default> SenderLock<T> {
    /// Creates a new `SenderLock` around the given sender.
    pub fn new(sender: Sender<T>) -> Self {
        Self {
            sender,
            is_owned: Mutex::new(false),
        }
    }

    /// Subscribes to the sender.
    pub fn subscribe(&self) -> Receiver<T> {
        self.sender.subscribe()
    }

    /// Tries to acquire ownership of the Sender.
    pub fn try_own(self: Arc<Self>) -> Option<OwnedSender<T>> {
        let mut is_owned = self.is_owned.lock().unwrap();
        if *is_owned {
            None
        } else {
            *is_owned = true;
            drop(is_owned);
            Some(OwnedSender { lock: self })
        }
    }

    /// Releases ownership of the Sender and resets the value to the default.
    fn free(&self) {
        self.sender.send_replace(T::default());
        *self.is_owned.lock().unwrap() = false;
    }
}

/// Owned RAII structure used to release ownership of the variable when dropped.
pub(crate) struct OwnedSender<T: Default> {
    lock: Arc<SenderLock<T>>,
}

impl<T: Default> OwnedSender<T> {
    /// Sends a new value to the Sender.
    pub fn send(&self, value: T) {
        self.lock.sender.send_replace(value);
    }
}

impl<T: Default> Drop for OwnedSender<T> {
    fn drop(&mut self) {
        self.lock.free();
    }
}
