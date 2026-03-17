use crate::msg::Message;

pub fn new(payload: Vec<u8>) -> Message {
    Message::ContentCount {
        content_id: u16::from_be_bytes([payload[0], payload[1]]),
    }
}

pub fn new_ack(payload: Vec<u8>) -> Message {
    let mut data: [u16; 24] = [0; 24];

    for i in 0..24 {
        let j = i * 2 + 3;
        data[i] = u16::from_be_bytes([payload[j], payload[j + 1]]);
    }

    Message::AckContentCount {
        content_id: u16::from_be_bytes([payload[0], payload[1]]),
        error: payload[2],
        data,
    }
}
