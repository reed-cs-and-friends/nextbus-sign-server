use crate::msg::Message;

pub fn new(payload: Vec<u8>) -> Message {
    Message::AckContent {
        content_id: u16::from_be_bytes([payload[0], payload[1]]),
        error: payload[2],
    }
}
