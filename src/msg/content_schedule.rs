use crate::msg::Message;

// Number of schedules ranges [0x00, 0xFF), freeing up
// 0xFF for indefinite schedules (always on).
pub const MAX_SCHEDULES: u8 = u8::MAX - 1;

pub const INDEFINITE_CODE: u8 = u8::MAX;

pub fn new(payload: Vec<u8>) -> Message {
    assert!(payload.len() >= 3, "invalid payload length");

    let content_id = u16::from_be_bytes([payload[0], payload[1]]);
    let num_deltas = payload[2];

    if num_deltas == INDEFINITE_CODE {
        return new_indefinite_content_schedule(content_id);
    }

    let mut schedules = Vec::new();
    if num_deltas == 0 {
        return new_content_schedule(content_id, schedules);
    }

    assert!(
        payload.len() >= 7 + usize::from(num_deltas) * 4,
        "invalid payload length"
    );

    let base_time_s = u32::from_be_bytes([payload[3], payload[4], payload[5], payload[6]]);
    let base_time_ms = u64::from(base_time_s) * 1_000;

    for sched in payload[7..].chunks(4).take(num_deltas.into()) {
        let start_dt_mins = u16::from_be_bytes([sched[0], sched[1]]);
        let stop_dt_mins = u16::from_be_bytes([sched[2], sched[3]]);

        let start_dt_ms = u64::from(start_dt_mins) * 60_000;
        let stop_dt_ms = u64::from(stop_dt_mins) * 60_000;

        schedules.push(Schedule {
            start: base_time_ms + start_dt_ms,
            stop: base_time_ms + stop_dt_ms,
        })
    }

    new_content_schedule(content_id, schedules)
}

pub fn new_ack(payload: Vec<u8>) -> Message {
    Message::AckContentSchedule {
        content_id: u16::from_be_bytes([payload[0], payload[1]]),
        error: payload[2],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Schedule {
    // both milliseconds, presumably
    pub start: u64,
    pub stop: u64,
}

fn new_content_schedule(content_id: u16, schedules: Vec<Schedule>) -> Message {
    // `min_time` is the minimum start time across all schedules.
    let min_time = schedules
        .iter()
        .map(|Schedule { start, .. }| *start)
        .min()
        .unwrap_or(u64::MAX);

    Message::ContentSchedule {
        content_id,
        min_time,
        start_stop_times: Some(schedules),
    }
}

fn new_indefinite_content_schedule(content_id: u16) -> Message {
    Message::ContentSchedule {
        content_id,
        min_time: u64::MAX,
        start_stop_times: None,
    }
}
