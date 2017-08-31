extern crate rumqtt_coroutines;
extern crate futures_await as futures;
extern crate mqtt3;

use std::thread;
use std::time::Duration;
use std::sync::Arc;

// use futures::sync::mpsc;
use std::sync::mpsc;
use futures::{Future, Sink};
use mqtt3::*;

use rumqtt_coroutines::NetworkRequest;

fn main() {
    let (command_tx, command_rx) = mpsc::channel();

    thread::spawn(move || {
        rumqtt_coroutines::start(command_tx);
    });

    let mut user_command_tx = command_rx.recv().unwrap(); 

    for i in 0..100 {
        let publish = Publish {
            dup: false,
            qos: QoS::AtLeastOnce,
            retain: false,
            topic_name: "hello/world".to_string(),
            pid: None,
            payload: Arc::new(vec![1, 2, 3])
        };
        let publish = NetworkRequest::Publish(publish);
        user_command_tx = match user_command_tx.send(publish).wait() {
            Ok(tx) => tx,
            Err(_) => {
                let mut user_command_tx = command_rx.recv().unwrap();
                user_command_tx
            }
        };

        thread::sleep(Duration::new(3, 0));
    }
}