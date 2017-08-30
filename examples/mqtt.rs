extern crate rumqtt_coroutines;

use std::thread;
use std::mpsc;

fn main() {
    let (command_tx, command_rx) = mpsc::channel(1000);
    
    thread::spawn(move || {
        for i in 0..100 {
            command_tx.send(i).wait()
        }
    });

    rumqtt_coroutines::start(command_rx);
}