use crate::msg::Message;

pub fn new(payload: Vec<u8>) -> Message {
    let conf_code = payload[0];
    let address = [payload[1], payload[2], payload[3], payload[4]];
    let port = u16::from_be_bytes([payload[5], payload[6]]);

    Message::AuthConfirm {
        conf_code,
        address,
        port,
    }
}
