use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

use anyhow::{Result, bail};
use chrono::{FixedOffset, Offset};
use crossbeam::channel::{self, select};
use env_logger;
use log;
use nextbus_sign_server::msg::{Message, content::PayloadType};
use rand::{Rng, rng};
use rouille::Response;

fn main() {
    env_logger::init();
    if let Err(e) = inner() {
        log::error!("Fatal error: {e:?}");
        std::process::exit(1);
    }

    fn inner() -> Result<()> {
        let (s, r) = channel::unbounded();
        let r = Arc::new(r);
        let s = Arc::new(s);

        thread::spawn(move || {
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

        let sp = s.clone();
        thread::spawn(move || {
            loop {
                if let Err(e) = sp.send(Instruction::Sync) {
                    log::error!("Failed to instruct sync: {e}");
                }
                thread::sleep(Duration::from_mins(1));
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

                if let Err(e) = s.send(Instruction::SetText(text)) {
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

#[derive(Clone, Copy)]
struct ClockMark {
    epoch_sec: u32, // 2038 will never happen.
    seq_num: u8,
    offset: FixedOffset,
}

enum Instruction {
    SetText(String),
    Sync,
}

fn handle(stream: TcpStream, msg_ch: Arc<channel::Receiver<Instruction>>) -> Result<()> {
    let addr = stream.peer_addr()?;
    log::info!("Handling connection from: {addr}");

    let (s, r) = nextbus_sign_server::run(stream);

    let mut clk_mark = None;

    loop {
        select!(
            recv(msg_ch) -> msg => {
                match msg {
                    Ok(Instruction::SetText(msg)) => {
                        if let Err(e) = s.send(Message::ContentMsg {
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
                        }) {
                            log::error!("Failed setting message: {msg}: {e}");
                        }
                    },
                    Ok(Instruction::Sync) => {
                        log::info!("Requesting clock mark.");

                        let time = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                            Err(e) => {
                                log::warn!("Epcoh was not before current time: {e}. Skipping clock sync.");
                                continue;
                            },
                            Ok(time) => time.as_secs().try_into().unwrap_or_else(|_| {
                                log::warn!("Time overflowing 32 bit repr.");
                                time.as_secs() as u32
                            }),
                        };

                        clk_mark = Some(ClockMark {
                            seq_num: rng().next_u32() as u8,
                            epoch_sec: time,
                            offset: chrono::Local::now().offset().fix(),
                        });

                        if let Err(e) = s.send(Message::MarkClock { sequence: clk_mark.unwrap().seq_num }) {
                            log::error!("Failed to send MarkClock: {e}");
                        }
                    },
                    Err(e) => log::error!("Failed to receive text message from channel: {e}"),
                }
            },
            recv(r) -> msg => match msg {
                Ok(msg) => {
                    log::info!("Recv'd: {msg:?}");
                    if let Some(resp) = respond_to(msg, clk_mark) {
                        if let Err(e) = s.send(resp) {
                            log::error!("Failed to send sign message to channel: {e}");
                        };
                    }
                },
                Err(e) => {
                    log::error!("Failed to receive sign message from channel: {e}");
                    bail!("Sign server terminated.")
                }
            },
        );
    }
}

fn respond_to(msg: Message, clk_mark: Option<ClockMark>) -> Option<Message> {
    match msg {
        Message::Ping { seq_num } => Some(Message::Pong { seq_num }),
        Message::AckMarkClock { seq_num } => match clk_mark {
            None => {
                log::warn!("Received unrequested AckMarkClock.");
                None
            }
            Some(ClockMark { seq_num: x, .. }) if x != seq_num => {
                log::warn!("Wrong MarkClock Ack'd: Saw {seq_num}, expected {x}");
                None
            }
            Some(mark) => {
                Some(Message::SyncClock {
                    epoch_time_sec: mark.epoch_sec,
                    seq_num,
                    tz: format!("GMT-{}", mark.offset),
                    zone_offset: 0, // unused so far as I can tell
                })
            }
        },
        _ => None,
    }
}
