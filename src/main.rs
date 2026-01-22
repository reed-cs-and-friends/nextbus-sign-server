use nextbus_sign_server::msg::{self, Message};
use std::io;
use std::net::{TcpListener, TcpStream};
use std::thread;

use anyhow::Result;
use env_logger;
use log;

fn main() {
    env_logger::init();

    if let Err(e) = inner() {
        log::error!("Fatal error: {e:?}");
        std::process::exit(1);
    }

    fn inner() -> Result<()> {
        let listener = TcpListener::bind("0.0.0.0:4502")?;

        for stream in listener.incoming() {
            match stream {
                Err(e) => log::warn!("Can't get stream: {e}"),
                Ok(c) => {
                    thread::spawn(move || {
                        if let Err(e) = handle(c) {
                            log::warn!("Couldn't handle connection: {e}");
                        }
                    });
                }
            }
        }

        Ok(())
    }
}

fn handle(stream: TcpStream) -> io::Result<()> {
    let addr = stream.peer_addr()?;
    log::info!("Handling connection from: {addr}");

    let (s, r) = nextbus_sign_server::run(stream);

    s.send(Message::ContentMsg {
        content_id: 0xff,
        content_channel: 2,
        count_impressions: false,
        display_indefinitely: true,
        booking_id: 0,
        priority: 0,
        payloads: vec![(
            msg::content::PayloadType::Msg,
            "chomp :3".as_bytes().to_vec(),
        )],
    })
    .unwrap();

    for msg in r {
        log::info!("ljk;sedf: {msg:?}");
    }

    Ok(())
}
