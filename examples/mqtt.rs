extern crate rumqtt_coroutines;
extern crate futures_await as futures;
extern crate mqtt3;
extern crate multiqueue;

use std::thread;
use std::time::Duration;
use std::sync::Arc;

use futures::sync::mpsc;
use futures::{Future, Sink};
use mqtt3::*;
use multiqueue::mpmc_fut_queue;

use rumqtt_coroutines::NetworkRequest;

fn main() {
    let (mut command_tx, command_rx) = mpmc_fut_queue(1000);
    
    let mut user_command_tx = command_tx.clone();

    thread::spawn(move || {
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

            user_command_tx = user_command_tx.send(publish).wait().unwrap();
            thread::sleep(Duration::new(3, 0));
        }
    });

    rumqtt_coroutines::start(command_rx, command_tx);
}