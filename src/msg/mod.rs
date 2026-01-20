mod app_running;
mod content;
mod debug;
mod pong;
mod reboot;

use std::io::Read;

use thiserror::Error;

use crate::msg::app_running::AppRunningReason;

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("Failed i/o: {0}")]
    Io(#[from] std::io::Error),
    #[error("Checksum mismatch. Decoded {0}, calculated {1}")]
    ChecksumMismatch(u16, u16),
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
            x => todo!("unknown type: {x}"),
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
            Reboot => 6,
            DebugMsg { .. } => 28,
            ShellCommand { .. } => 80,
            ContentMsg { .. } => 32,
            _ => todo!(),
        }
    }

    pub fn get_payload(&self) -> Vec<u8> {
        use Message::*;
        match self {
            Ping { seq_num } => vec![*seq_num],
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
            _ => todo!(),
        }
    }
}
