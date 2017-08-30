#![feature(proc_macro, conservative_impl_trait, generators)]

extern crate futures_await as futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_timer;
extern crate mqtt3;
extern crate bytes;
#[macro_use]
extern crate log;
extern crate multiqueue;

mod codec;
mod packet;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::io::{self, ErrorKind};
use std::error::Error;
use std::time::Duration;
use std::thread;

use codec::MqttCodec;

use futures::prelude::*;
use futures::stream::{Stream, SplitSink};
// use futures::sync::mpsc::{Sender, Receiver};
use tokio_core::reactor::{Core, Handle};
use tokio_core::net::TcpStream;
use tokio_timer::Timer;
use tokio_io::AsyncRead;
use tokio_io::codec::Framed;
use multiqueue::{MPMCFutSender, MPMCFutReceiver};
use mqtt3::*;

#[derive(Debug, Clone)]
pub enum NetworkRequest {
    Subscribe(Vec<(TopicPath, QoS)>),
    Publish(Publish),
    Ping,
}

pub fn start(commands_rx: MPMCFutReceiver<NetworkRequest>, commands_tx: MPMCFutSender<NetworkRequest>) {
    // let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1883);

    loop {
        let mut reactor = Core::new().unwrap();
        let commands_tx = commands_tx.clone();
        let commands_rx = commands_rx.clone();
        let handle = reactor.handle();

        let client = async_block! {
            let (framed, handle) = await!(try_reconnect(handle))?;
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

            // CON: Error should be clearer if this async block is missed
            // ^^^ the trait `std::ops::Generator` is not implemented for `[closure@src/lib.rs:62:22: 1:5 handle:_, commands_tx:_]`
            #[async]
            for msg in receiver {
                println!("message = {:?}", msg);
            }
            println!("Done with network receiver !!");

            // CON: Error should be clean when async_block doesn't return 'Result'
            // ^^^ the `?` operator can only be used in a function that returns `Result` (or another type that implements `std::ops::Try`)
            Ok::<(), io::Error>(())
        };

        let _response = reactor.run(client);
        println!("{:?}", _response);
    }
}

// CHECK: Can't return Result<Framed<TcpStream, MqttCodec>, Error>
// fn try_reconnect(reactor: &mut Core) -> io::Result<Framed<TcpStream, MqttCodec>> {
//     let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1883);
//     let connect_packet = packet::gen_connect_packet("rumqtt-coro", 10, true, None, None);

//     let f_response = TcpStream::connect(&addr, &reactor.handle()).and_then(|connection| {
//         let framed = connection.framed(MqttCodec);
//         let f1 = framed.send(connect_packet);

//         f1.and_then(|framed| {
//                 framed.into_future()
//                     .and_then(|(res, stream)| Ok((res, stream)))
//                     .map_err(|(err, _stream)| err)
//             })
//     });

//     let response = reactor.run(f_response);
//     // TODO: Check ConnAck Status and Error out incase of failure
//     let (_packet, frame) = response?;
//     Ok(frame)
// }

#[async]
fn try_reconnect(handle: Handle) -> io::Result<(Framed<TcpStream, MqttCodec>, Handle)> {
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1883);

        let stream = await!(TcpStream::connect(&address, &handle))?;
        let connect = packet::gen_connect_packet("rumqtt-coro", 10, true, None, None);

        let framed = stream.framed(MqttCodec);
        let framed = await!(framed.send(connect)).unwrap();

        Ok((framed, handle))
}

#[async]
fn ping_timer(mut commands_tx: MPMCFutSender<NetworkRequest>) -> io::Result<()> {
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
fn command_read(commands_rx: MPMCFutReceiver<NetworkRequest>, mut sender: SplitSink<Framed<TcpStream, MqttCodec>>) -> io::Result<()> {

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
