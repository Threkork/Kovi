use std::any::Any;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use tokio::sync::mpsc;

pub struct Oneshot<T> {
    sender: mpsc::Sender<T>,
    receiver: mpsc::Receiver<T>,
}



pub fn channel<T: 'static + Send>(id: &str) -> (mpsc::Sender<T>, mpsc::Receiver<T>) {
    let mut channels = CHANNELS.lock().unwrap();

    if let Some(channel) = channels.get(id) {
        let sender = channel.sender.clone();
        let receiver = channel.receiver.clone();
        (
            sender.downcast_sender().unwrap(),
            receiver.downcast_receiver().unwrap(),
        )
    } else {
        let (sender, receiver) = mpsc::channel(32);
        let channel = Channel { sender, receiver };
        channels.insert(id.to_string(), channel);
        (sender, receiver)
    }
}
