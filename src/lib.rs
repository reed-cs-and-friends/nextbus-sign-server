use std::io::Write;
use std::net::TcpStream;

use crate::msg::Message;
use crossbeam::channel;
use log;

pub mod msg;

/// Wrap a sign to provide channels for messages. Anything sent will be written, and anything
/// received will be sent.
pub fn run(
    stream: TcpStream,
) -> (
    channel::Sender<msg::Message>,
    channel::Receiver<msg::Message>,
) {
    let mut reader_stream = stream.try_clone().unwrap();
    let mut writer_stream = stream.try_clone().unwrap();

    let (send_parsed_from_tcp, recv_parsed_from_tcp) = channel::unbounded();
    let (send_to_tcp, recv_to_tcp) = channel::unbounded::<Message>();

    // Reader thread from TCP.
    std::thread::spawn(move || {
        loop {
            let msg = match Message::decode(&mut reader_stream) {
                Ok(m) => m,
                Err(e) => {
                    log::error!("Failed reading from TCP stream: {e}");
                    continue;
                }
            };

            if let Err(e) = send_parsed_from_tcp.send(msg) {
                log::warn!("Failed to send msg over channel: {e}");
            }
        }
    });

    // Writer thread to TCP.
    std::thread::spawn(move || {
        for msg in recv_to_tcp.into_iter() {
            log::info!("Sending: {msg:?}");

            let msg = msg.encode();
            if let Err(e) = (&mut writer_stream).write_all(&msg) {
                log::error!("Failed to send msg over TCP: {e}");
            };
        }
    });

    (send_to_tcp, recv_parsed_from_tcp)
}
