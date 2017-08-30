#![feature(proc_macro, conservative_impl_trait, generators)]

extern crate futures_await as futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_timer;
extern crate mqtt3;
extern crate bytes;
#[macro_use]
extern crate log;

mod codec;
mod packet;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::io::{self, ErrorKind};
use std::error::Error;
use std::time::Duration;

use codec::MqttCodec;

use futures::prelude::*;
use futures::stream::{Stream, SplitSink};
use futures::sync::mpsc::{Sender, Receiver};
use tokio_core::reactor::Core;
use tokio_core::net::TcpStream;
use tokio_timer::Timer;

use tokio_io::AsyncRead;
use tokio_io::codec::Framed;
use mqtt3::*;

#[derive(Debug)]
pub enum NetworkRequest {
    Subscribe(Vec<(TopicPath, QoS)>),
    Publish(Publish),
    Ping,
}

pub fn start(commands_rx: Receiver<NetworkRequest>, commands_tx: Sender<NetworkRequest>) {
    let mut reactor = Core::new().unwrap();
    let handle = reactor.handle();

    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1883);

    let client = async_block! {
        let stream = await!(TcpStream::connect(&address, &handle)).unwrap();
        let connect = packet::gen_connect_packet("rumqtt-coro", 10, true, None, None);

        let framed = stream.framed(MqttCodec);
        let framed = await!(framed.send(connect)).unwrap();

        let (sender, receiver) = framed.split();

        // ping timer
        handle.spawn(ping_timer(commands_tx).then(|result| {
            match result {
                Ok(_) => println!("Ping timer done"),
                Err(e) => println!("Ping timer IO error {:?}", e),
            }
            Ok(())
        }));

        // network transmission requests
        handle.spawn(command_read(commands_rx, sender).then(|result| {
            match result {
                Ok(_) => println!("Command receiver done"),
                Err(e) => println!("Command IO error {:?}", e),
            }
            Ok(())
        }));

        // incoming network messages
        #[async]
        for msg in receiver {
            println!("message = {:?}", msg);
        }
        println!("Done with network receiver !!");
        Ok::<(), io::Error>(())
    };

    let _response = reactor.run(client);
    println!("{:?}", _response);
}

#[async]
fn ping_timer(mut commands_tx: Sender<NetworkRequest>) -> io::Result<()> {
    let timer = Timer::default();
    let keep_alive = 10;
    let interval = timer.interval(Duration::new(keep_alive, 0));

    #[async]
    for _t in interval {
        println!("Ping timer fire");
        commands_tx = await!(
            commands_tx.send(NetworkRequest::Ping).or_else(|e| {
                Err(io::Error::new(ErrorKind::Other, e.description()))
            })
        )?;
    }

    Ok(())
}

#[async]
fn command_read(commands_rx: Receiver<NetworkRequest>, mut sender: SplitSink<Framed<TcpStream, MqttCodec>>) -> io::Result<()> {

    let commands_rx = commands_rx.or_else(|_| {
            Err(io::Error::new(ErrorKind::Other, "Rx Error"))
    });
    
    #[async]
    for command in commands_rx {
        println!("command = {:?}", command);
        let packet = match command {
            NetworkRequest::Publish(publish) => {
                Packet::Publish(publish)
            }
            NetworkRequest::Ping => {
                packet::gen_pingreq_packet()
            }
            _ => unimplemented!()
        };

        sender = await!(sender.send(packet))?
    }

    Ok(())
}
