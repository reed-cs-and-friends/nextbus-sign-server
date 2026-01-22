use crossbeam::channel::{self, select};
use nextbus_sign_server::msg::{Message, content::PayloadType};
use rouille::Response;
use std::io::{self, Read};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
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
        let (s, r) = channel::unbounded();
        let r = Arc::new(r);

        let sign_handler_t = thread::spawn(move || {
            let listener = TcpListener::bind("0.0.0.0:4502").unwrap();

            for stream in listener.incoming() {
                match stream {
                    Err(e) => log::warn!("Can't get stream: {e}"),
                    Ok(c) => {
                        let r = r.clone();
                        thread::spawn(move || {
                            if let Err(e) = handle(c, r) {
                                log::warn!("Couldn't handle connection: {e}");
                            }
                        });
                    }
                }
            }
        });

        rouille::start_server("0.0.0.0:8080", move |request| {
            if request.method() == "POST" && request.url() == "/write" {
                let Some(mut body) = request.data() else {
                    return Response::text("Request body must be sent.").with_status_code(500);
                };

                let mut text = String::new();
                if let Err(e) = body.read_to_string(&mut text) {
                    log::warn!("Can't read request body: {e}");
                    return Response::text("Can't read request body.").with_status_code(500);
                };

                if let Err(e) = s.send(text) {
                    log::error!("Channel sending error: {e}");
                    return Response::text("Can't send over channel.").with_status_code(500);
                };

                Response::empty_404().with_status_code(200) // no empty_200() response lol
            } else {
                Response::text("Only route is POST /write.").with_status_code(404)
            }
        })
    }
}

fn handle(stream: TcpStream, msg_ch: Arc<channel::Receiver<String>>) -> io::Result<()> {
    let addr = stream.peer_addr()?;
    log::info!("Handling connection from: {addr}");

    let (s, r) = nextbus_sign_server::run(stream);

    loop {
        select!(
            recv(msg_ch) -> msg => {
                match msg {
                    Ok(msg) => {
                        s.send(Message::ContentMsg {
                            content_id: 0x11,
                            content_channel: 2,
                            count_impressions: false,
                            display_indefinitely: true,
                            booking_id: 0,
                            priority: 0,
                            payloads: vec![(
                                PayloadType::Msg,
                                msg.as_bytes().to_vec(),
                            )],
                        })
                        .unwrap();
                    },
                    Err(e) => log::error!("Failed to receive text message from channel: {e}"),
                }
            }
            recv(r) -> msg => {
                log::info!("Recv'd: {msg:?}");
            }
        );
    }
}
