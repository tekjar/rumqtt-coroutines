extern crate rumqtt_coroutines;
extern crate futures_await as futures;

use std::thread;
use futures::sync::mpsc;
use futures::{Future, Sink};

fn main() {
    let (mut command_tx, command_rx) = mpsc::channel(1000);
    
    thread::spawn(move || {
        for i in 0..100 {
            command_tx = command_tx.send(i).wait().unwrap();
        }
    });

    rumqtt_coroutines::start(command_rx);
}