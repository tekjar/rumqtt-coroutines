#![feature(proc_macro, conservative_impl_trait, generators)]

extern crate futures_await as futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate mqtt3;
extern crate bytes;
#[macro_use]
extern crate log;

mod codec;
mod packet;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::io::{self, Cursor, ErrorKind};

use codec::MqttCodec;

use futures::prelude::*;
use futures::stream::{Stream, SplitStream};
use futures::sync::mpsc::{Receiver};
use tokio_core::reactor::Core;
use tokio_core::net::TcpStream;
use tokio_io::AsyncRead;
use tokio_io::codec::Framed;
use mqtt3::*;

#[derive(Debug)]
pub enum NetworkRequest {
    Subscribe(Vec<(TopicPath, QoS)>),
    Publish(Publish),
}

pub fn start(commands: Receiver<NetworkRequest> ) {
    let mut reactor = Core::new().unwrap();
    let handle = reactor.handle();

    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1883);

    let client = async_block! {
        let stream = await!(TcpStream::connect(&address, &handle)).unwrap();
        let connect = packet::gen_connect_packet("rumqtt-coro", 10, true, None, None);

        let framed = stream.framed(MqttCodec);
        let framed = await!(framed.send(connect)).unwrap();

        let (mut sender, receiver) = framed.split();

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


        let commands = commands.or_else(|_| {
            Err(io::Error::new(ErrorKind::Other, "Rx Error"))
        });

        #[async]
        for command in commands {
            println!("command = {:?}", command);

            match command {
                NetworkRequest::Publish(publish) => {
                    let publish = Packet::Publish(publish);
                    sender = await!(sender.send(publish))?
                }
                _ => unimplemented!()
            }

        }

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
// fn command_read(commands: Receiver<i32>) -> Result<i32, ()> {
//     #[async]
//     for command in commands {
//         println!("command = {:?}", command);
//     }

//     Ok(100)
// }
