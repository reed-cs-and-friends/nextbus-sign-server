use crate::msg::Message;

pub fn new(payload: Vec<u8>) -> Message {
    Message::AuthRequest { method: payload[0] }
}
