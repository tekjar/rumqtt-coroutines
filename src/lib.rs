#![feature(proc_macro, conservative_impl_trait, generators)]

extern crate futures_await as futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate mqtt3;
extern crate bytes;
#[macro_use]
extern crate log;

mod codec;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::io;

use codec::MqttCodec;

use futures::prelude::*;
use futures::stream::{Stream, SplitStream};
use futures::sync::mpsc::{self, Receiver, Sender};
use tokio_core::reactor::Core;
use tokio_core::net::TcpStream;
use tokio_io::AsyncRead;
use tokio_io::codec::Framed;

pub fn start(commands: Receiver<i32> ) {
    let mut reactor = Core::new().unwrap();
    let handle = reactor.handle();

    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1883);

    let client = async_block! {
        let stream = await!(TcpStream::connect(&address, &handle)).unwrap();
        let framed = stream.framed(MqttCodec);

        let (sender, receiver) = framed.split();

        handle.spawn(mqtt_recv(receiver).then(|result| {
            match result {
                Ok(_) => println!("Network receiver done"),
                Err(e) => println!("Network IO error {:?}", e),
            }
            Ok(())
        }));

        // handle.spawn(command_read(commands).then(|result| {
        //     match result {
        //         Ok(_) => println!("Command receiver done"),
        //         Err(e) => println!("Command IO error {:?}", e),
        //     }
        //     Ok(())
        // }));

        // async_block! {
        //     for command in commands {
        //         println!("command = {:?}", command);
        //     }
        //     Ok(())
        // }

        Ok::<(), io::Error>(())
    };

    let _response = reactor.run(client).unwrap();
}

#[async]
fn mqtt_recv(receiver: SplitStream<Framed<TcpStream, MqttCodec>>) -> io::Result<()> {
    #[async]
    for msg in receiver {
        println!("message = {:?}", msg);
    }

    Ok(())
}

// #[async]
// fn command_read(commands: Receiver<i32>) -> io::Result<()> {
//     #[async]
//     for command in commands {
//         println!("command = {:?}", command);
//     }

//     Ok(())
// }
