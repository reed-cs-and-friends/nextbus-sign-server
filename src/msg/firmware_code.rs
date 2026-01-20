use crate::msg::Message;

pub fn new(mut payload: Vec<u8>) -> Message {
    let seq = payload[0];
    let dest_addr = u16::from_be_bytes([payload[1], payload[2]]);
    let num_bytes = u16::from_be_bytes([payload[3], payload[4]]);
    let code_chunk = {
        payload.drain(0..5);
        payload
    };
    assert_eq!(
        code_chunk.len(),
        num_bytes.into(),
        "reported code chunk size is a lie"
    );
    Message::FirmwareCode {
        seq,
        dest_addr,
        num_bytes,
        code_chunk,
    }
}
