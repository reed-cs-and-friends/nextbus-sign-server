use nextbus_sign_server::Message;
use std::io::{self, Write};
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

fn handle(mut stream: TcpStream) -> io::Result<()> {
    let addr = stream.peer_addr()?;
    log::info!("Handling connection from: {addr}");

    let msg = Message::ShellCommand {
        command_id: 0,
        command: "reboot".to_string(),
    }
    .encode();
    stream.write_all(&msg)?;

    loop {
        let msg = Message::decode(&mut stream).expect("lol");
        log::info!("ljk;sedf: {msg:?}");
    }
}
