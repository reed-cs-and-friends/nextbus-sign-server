use crate::msg::Message;

pub fn new_sync(payload: Vec<u8>) -> Message {
    let seq_num = payload[0];
    let epoch_time_sec = u32::from_be_bytes([payload[1], payload[2], payload[3], payload[4]]);
    let zone_offset = payload[5];
    let tz_len = payload[6] as usize;
    let tz = String::from_utf8(payload[7..(7 + tz_len)].to_vec()).unwrap_or_else(|e| {
        log::warn!("Couldn't parse given TZ as UTF-8: {e}. Defaulting to GMT.");
        "GMT".to_string()
    });
    Message::SyncClock {
        seq_num,
        epoch_time_sec,
        zone_offset,
        tz,
    }
}

pub fn new_sync_ack(payload: Vec<u8>) -> Message {
    let mark_id = payload[0];
    let error = payload[1];
    let drift_sec = u16::from_be_bytes([payload[2], payload[3]]);
    Message::AckSyncClock {
        mark_id,
        error,
        drift_sec,
    }
}

pub fn new_mark(payload: Vec<u8>) -> Message {
    Message::MarkClock {
        sequence: payload[0],
    }
}

pub fn new_mark_ack(payload: Vec<u8>) -> Message {
    Message::AckMarkClock {
        seq_num: payload[0],
    }
}
