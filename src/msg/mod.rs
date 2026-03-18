mod ack_content;
mod app_running;
mod auth_confirm;
mod auth_request;
mod cfg_params;
mod clock;
pub mod content;
pub mod content_count;
pub mod content_delete;
pub mod content_schedule;
mod debug;
mod firmware_code;
pub mod ping;
mod pong;
mod reboot;
mod stop_cfg;

use std::io::Read;

use thiserror::Error;

use crate::msg::app_running::AppRunningReason;

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("Failed i/o: {0}")]
    Io(#[from] std::io::Error),
    #[error("Checksum mismatch. Decoded {0}, calculated {1}")]
    ChecksumMismatch(u16, u16),
    #[error("Unknown message of type: {0}")]
    UnknownMessage(u8),
}

#[derive(Debug)]
pub enum Message {
    Ping {
        seq_num: u8,
    },
    Pong {
        seq_num: u8,
    },
    AppRunning {
        seq_num: u8,
        reason: AppRunningReason,
    },
    Reboot,
    DebugMsg {
        msg: String,
    },
    ShellCommand {
        command: String,
        command_id: u8,
    },
    ContentMsg {
        content_id: u16,
        content_channel: u8,
        count_impressions: bool,
        display_indefinitely: bool,
        booking_id: u16,
        priority: u16,
        payloads: Vec<(content::PayloadType, Vec<u8>)>,
    },
    AckContent {
        content_id: u16,
        error: u8,
    },
    ContentDelete {
        content_id: u16,
    },
    AckContentDelete {
        content_id: u16,
        error: u8,
    },
    ContentCount {
        content_id: u16,
    },
    AckContentCount {
        content_id: u16,
        error: u8,
        data: [u16; 24],
    },
    ContentSchedule {
        content_id: u16,
        min_time: u64,
        /// Note: a value of None indicates that the content is scheduled indefinitely.
        /// A length of zero indicates that the content is never scheduled.
        start_stop_times: Option<Vec<content_schedule::Schedule>>,
    },
    AckContentSchedule {
        content_id: u16,
        error: u8,
    },
    SyncClock {
        seq_num: u8,
        epoch_time_sec: u32,
        zone_offset: u8,
        tz: String,
    },
    AckSyncClock {
        mark_id: u8,
        error: u8,
        drift_sec: u16,
    },
    MarkClock {
        sequence: u8,
    },
    AckMarkClock {
        seq_num: u8,
    },
    FirmwareCode {
        seq: u8,
        dest_addr: u16,
        num_bytes: u16,
        code_chunk: Vec<u8>,
    },
    AuthRequest {
        method: u8,
    },
    AuthConfirm {
        conf_code: u8,
        address: [u8; 4],
        port: u16,
    },
    GetCfgParam {
        param: u8,
    },
    AckGetCfgParam {
        param: u8,
        error: u8,
        value: u8,
    },
    SetCfgParam {
        param: u8,
        value: u8,
    },
    AckSetCfgParam {
        param: u8,
        error: u8,
        value: u8,
    },
    ResetCfgParams,
    AckResetCfgParams,
    StopCfg {
        stop_id: u8,
        title: String,
        phoneme: String,
        route_tag: String,
        snd_md5: String,
        snd_url: String,
        zero_countdown_msg: String,
    },
    AckStopCfg {
        stop_id: u8,
        error: u8,
    },
    ClearStopCfg,
    AckClearStopCfg,
}

impl Message {
    pub fn decode<I: Read>(mut stream: I) -> Result<Self, DecodeError> {
        let mut t = [0];
        stream.read_exact(&mut t)?;
        let t = t[0];

        let mut len_bytes = [0, 0];
        stream.read_exact(&mut len_bytes)?;
        let len = u16::from_be_bytes(len_bytes);

        assert!(len >= 5);
        let mut payload = vec![0; (len - 5) as usize];
        stream.read_exact(&mut payload)?;
        let payload = payload;

        let mut cksum = [0; 2];
        stream.read_exact(&mut cksum)?;
        let cksum = u16::from_be_bytes(cksum);

        let mut checked_message = vec![t, len_bytes[0], len_bytes[1]];
        checked_message.extend(&payload);
        let cksum_calc = Self::cksum(&checked_message);
        if cksum_calc != cksum {
            return Err(DecodeError::ChecksumMismatch(cksum, cksum_calc));
        }

        log::trace!("read: {t} {payload:?}");

        Ok(match t {
            8 => app_running::new(payload),
            11 => pong::new(payload),
            6 => reboot::new(),
            28 => debug::new(payload),
            32 => content::new(payload),
            31 => firmware_code::new(payload),
            33 => ack_content::new(payload),
            34 => content_schedule::new(payload),
            35 => content_schedule::new_ack(payload),
            36 => content_delete::new(payload),
            37 => content_delete::new_ack(payload),
            38 => content_count::new(payload),
            39 => content_count::new_ack(payload),
            50 => auth_request::new(payload),
            52 => auth_confirm::new(payload),
            26 => clock::new_sync(payload),
            27 => clock::new_sync_ack(payload),
            24 => clock::new_mark(payload),
            25 => clock::new_mark_ack(payload),
            20 => cfg_params::new_get(payload),
            21 => cfg_params::new_get_ack(payload),
            18 => cfg_params::new_set(payload),
            19 => cfg_params::new_set_ack(payload),
            22 => cfg_params::new_reset(),
            23 => cfg_params::new_reset_ack(),
            14 => stop_cfg::new(payload),
            15 => stop_cfg::new_ack(payload),
            16 => stop_cfg::new_clear(),
            17 => stop_cfg::new_clear_ack(),
            10 => ping::new(payload),
            x => {
                return Err(DecodeError::UnknownMessage(x));
            }
        })
    }

    pub fn encode(self) -> Vec<u8> {
        let t = self.get_type();
        // type == Byte.MIN_VALUE is special-cased! otherwise, we get the payload and then frame
        // the command. (See Codec.java)

        let payload = self.get_payload();
        let len = (payload.len() + 5) as u16;
        let mut out = Vec::with_capacity(len as usize);
        out.push(t);

        out.extend(len.to_be_bytes());
        out.extend(payload);

        let cksum = Self::cksum(&out);
        out.extend(cksum.to_be_bytes());
        eprintln!("{out:?}");

        out
    }

    fn cksum(xs: &[u8]) -> u16 {
        let mut sum: u16 = 22218;
        for x in xs {
            let mut x = *x;

            for _ in 0..8 {
                if (((x as u16) ^ sum) & 1u16) != 0 {
                    sum = (sum >> 1) ^ 0x8408;
                } else {
                    sum >>= 1;
                }

                x >>= 1;
            }
        }

        sum
    }

    pub fn get_type(&self) -> u8 {
        use Message::*;

        match self {
            Ping { .. } => 10,
            Pong { .. } => 11,
            Reboot => 6,
            DebugMsg { .. } => 28,
            ShellCommand { .. } => 80,
            ContentMsg { .. } => 32,
            AckContent { .. } => 33,
            ContentDelete { .. } => 36,
            AckContentDelete { .. } => 37,
            ContentCount { .. } => 38,
            AckContentCount { .. } => 39,
            ContentSchedule { .. } => 34,
            AckContentSchedule { .. } => 35,
            MarkClock { .. } => 24,
            AckMarkClock { .. } => 25,
            SyncClock { .. } => 26,
            AckSyncClock { .. } => 27,
            FirmwareCode { .. } => 31,
            AuthRequest { .. } => 50,
            AuthConfirm { .. } => 52,
            GetCfgParam { .. } => 20,
            AckGetCfgParam { .. } => 21,
            SetCfgParam { .. } => 18,
            AckSetCfgParam { .. } => 19,
            ResetCfgParams => 22,
            AckResetCfgParams => 23,
            StopCfg { .. } => 14,
            AckStopCfg { .. } => 15,
            ClearStopCfg => 16,
            AckClearStopCfg => 17,
            _ => todo!(),
        }
    }

    pub fn get_payload(&self) -> Vec<u8> {
        use Message::*;
        match self {
            Ping { seq_num } => vec![*seq_num],
            Pong { seq_num } => vec![*seq_num],
            Reboot => vec![],
            DebugMsg { msg } => msg.clone().into_bytes(),
            ShellCommand {
                command,
                command_id,
            } => {
                let mut out = vec![*command_id];
                out.extend((command.len() as u16).to_be_bytes());
                out.extend(command.as_bytes());

                out
            }
            ContentMsg {
                content_id,
                content_channel,
                count_impressions,
                display_indefinitely,
                booking_id,
                priority,
                payloads,
            } => {
                let mut out = vec![];

                out.extend(content_id.to_be_bytes());
                out.push(*content_channel);

                let mut flags: u8 = 0;
                if *count_impressions {
                    flags |= 0x1;
                }
                if *display_indefinitely {
                    flags |= 0x2;
                }
                out.push(flags);

                out.extend(booking_id.to_be_bytes());
                out.extend(priority.to_be_bytes());

                out.push(payloads.len() as u8);
                for (t, p) in payloads {
                    out.push(*t as u8);
                    out.extend((p.len() as u16).to_be_bytes());
                    out.extend(p);
                }
                out
            }
            AckContent { content_id, error } => {
                let mut out = vec![];

                out.extend(content_id.to_be_bytes());
                out.push(*error);

                return out;
            }
            ContentDelete { content_id } => content_id.to_be_bytes().to_vec(),
            AckContentDelete { content_id, error } => {
                let mut out = vec![];

                out.extend(content_id.to_be_bytes());
                out.push(*error);

                return out;
            }
            ContentCount { content_id } => content_id.to_be_bytes().to_vec(),
            AckContentCount {
                content_id,
                error,
                data,
            } => {
                let mut out = vec![];

                out.extend(content_id.to_be_bytes());
                out.push(*error);
                out.extend(data.iter().fold(vec![], |mut acc, s| {
                    acc.extend(s.to_be_bytes().to_vec());
                    return acc;
                }));

                return out;
            }
            ContentSchedule {
                content_id,
                min_time,
                start_stop_times,
            } => {
                let mut out = Vec::new();
                out.extend(content_id.to_be_bytes());

                if let Some(start_stop_times) = start_stop_times {
                    let num_deltas = start_stop_times
                        .len()
                        .clamp(0, content_schedule::MAX_SCHEDULES.into());
                    out.push(num_deltas as u8);

                    if num_deltas != 0 {
                        let min_time_s = (min_time / 1_000) as u32;
                        out.extend(u32::to_be_bytes(min_time_s));

                        for content_schedule::Schedule { start, stop } in
                            start_stop_times.iter().take(num_deltas)
                        {
                            let start_dt_ms = start - min_time;
                            let stop_dt_ms = stop - min_time;

                            let start_dt_mins = (start_dt_ms / 60_000) as u16;
                            let stop_dt_mins = (stop_dt_ms / 60_000) as u16;

                            out.extend(u16::to_be_bytes(start_dt_mins));
                            out.extend(u16::to_be_bytes(stop_dt_mins));
                        }
                    }
                } else {
                    let num_deltas = content_schedule::INDEFINITE_CODE;
                    out.push(num_deltas);
                }

                out
            }
            AckContentSchedule { content_id, error } => {
                let mut out = Vec::new();
                out.extend(content_id.to_be_bytes());
                out.push(*error);
                out
            }
            MarkClock { sequence } => vec![*sequence],
            AckMarkClock { seq_num } => vec![*seq_num],
            SyncClock {
                seq_num,
                epoch_time_sec,
                zone_offset,
                tz,
            } => {
                let mut out = vec![*seq_num];
                out.extend(epoch_time_sec.to_be_bytes());
                out.push(*zone_offset);
                out.push(tz.len() as u8);
                out.extend(tz.as_bytes());
                out
            }
            AckSyncClock {
                mark_id,
                error,
                drift_sec,
            } => {
                let mut out = vec![*mark_id, *error];
                out.extend(drift_sec.to_be_bytes());
                out
            }
            FirmwareCode {
                seq,
                dest_addr,
                num_bytes,
                code_chunk,
            } => {
                let mut out = vec![*seq];
                out.extend(dest_addr.to_be_bytes());
                out.extend(num_bytes.to_be_bytes());
                out.extend(code_chunk);
                out
            }
            AuthRequest { method } => vec![*method],
            AuthConfirm {
                conf_code,
                address,
                port,
            } => {
                let mut out = vec![*conf_code];
                out.extend(address);
                out.extend(port.to_be_bytes());
                out
            }
            GetCfgParam { param } => vec![*param],
            AckGetCfgParam {
                param,
                error,
                value,
            } => vec![*param, *error, *value],
            SetCfgParam { param, value } => vec![*param, *value],
            AckSetCfgParam {
                param,
                error,
                value,
            } => vec![*param, *error, *value],
            ResetCfgParams => vec![],
            AckResetCfgParams => vec![],
            StopCfg {
                stop_id,
                title,
                phoneme,
                route_tag,
                snd_md5,
                snd_url,
                zero_countdown_msg,
            } => {
                let mut out = vec![*stop_id];

                out.push(title.len() as u8);
                out.push(phoneme.len() as u8);

                out.extend(title.as_bytes());

                out.extend(phoneme.as_bytes());

                out.push(zero_countdown_msg.len() as u8);
                out.extend(zero_countdown_msg.as_bytes());

                out.push(route_tag.len() as u8);
                out.extend(route_tag.as_bytes());

                out.push(snd_md5.len() as u8);
                out.extend(snd_md5.as_bytes());

                out.push(snd_url.len() as u8);
                out.extend(snd_url.as_bytes());

                out
            }
            AckStopCfg { stop_id, error } => vec![*stop_id, *error],
            ClearStopCfg => vec![],
            AckClearStopCfg => vec![],
            _ => todo!(),
        }
    }
}
