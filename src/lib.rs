use log;
use std::io::Read;

use thiserror::Error;

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
}

#[derive(Debug)]
pub enum AppRunningReason {
    Undiscernable,
    Powerup,
    Watchdog,
    ServerOrder,
    NewFirmware,
    NoServerContact,
    Redirected,
    DroppedConnection,
    BadAuthentication,
    FatalError,
    Unknown,
}

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("Failed i/o: {0}")]
    Io(#[from] std::io::Error),
    #[error("Checksum mismatch. Decoded {0}, calculated {1}")]
    ChecksumMismatch(u16, u16),
}

impl From<u8> for AppRunningReason {
    fn from(x: u8) -> Self {
        match x {
            0 => Self::Undiscernable,
            1 => Self::Powerup,
            2 => Self::Watchdog,
            3 => Self::ServerOrder,
            4 => Self::NewFirmware,
            5 => Self::NoServerContact,
            6 => Self::Redirected,
            7 => Self::DroppedConnection,
            8 => Self::BadAuthentication,
            9 => Self::FatalError,
            _ => Self::Unknown,
        }
    }
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
            8 => Self::AppRunning {
                seq_num: payload[0],
                reason: payload[1].into(),
            },
            11 => Self::Pong {
                seq_num: payload[0],
            },
            6 => Self::Reboot,
            28 => Self::DebugMsg {
                msg: String::from_utf8(payload).expect("Invalid payload in debug message!"),
            },
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
            _ => todo!(),
        }
    }
}
