use crate::msg::Message;

#[derive(Debug, Clone, Copy)]
pub enum PayloadType {
    Msg = 0,
    Phoneme = 1,
    SoundURL = 3,
    SoundChecksum = 4,
    RouteTags = 5,
    Bitmap = 2,
}

pub fn new(payload: Vec<u8>) -> Message {
    let content_id = u16::from_be_bytes([payload[0], payload[1]]);
    let content_channel = payload[2];
    let count_impressions = (payload[3] & 0x1) != 0;
    let display_indefinitely = (payload[3] & 0x2) != 0;
    let booking_id = u16::from_be_bytes([payload[4], payload[5]]);
    let priority = u16::from_be_bytes([payload[6], payload[7]]);
    let num_payloads = payload[8];

    let mut payloads: Vec<(PayloadType, Vec<u8>)> =
        vec![(PayloadType::Msg, vec![]); num_payloads.into()];
    let mut offset = 9;
    for i in 0..num_payloads {
        let typ = payload[offset];
        let len = u16::from_be_bytes([payload[offset + 1], payload[offset + 2]]);
        let p = payload[(offset + 3)..(offset + (len as usize) + 3)].to_vec();
        payloads[i as usize] = match typ {
            0 => (PayloadType::Msg, p),
            1 => (PayloadType::Phoneme, p),
            3 => (PayloadType::SoundURL, p),
            4 => (PayloadType::SoundChecksum, p),
            5 => (PayloadType::RouteTags, p),
            2 => (PayloadType::Bitmap, p),
            _ => panic!("Unexpected content payload type {}", i),
        };

        offset += (len as usize) + 3;
    }

    Message::ContentMsg {
        content_id,
        content_channel,
        count_impressions,
        display_indefinitely,
        booking_id,
        priority,
        payloads,
    }
}
