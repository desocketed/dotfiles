//! The control socket: `moonlander-leds set|get|status|reset` talk to the
//! running daemon over `$XDG_RUNTIME_DIR/moonlander-leds.sock`, one request
//! line in, the reply text out. Changes last until the daemon restarts.

use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::os::unix::net::UnixListener;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use anyhow::anyhow;
use clap::Subcommand;
use clap::ValueEnum;
use crossbeam_channel::Sender;
use log::warn;

use crate::Update;
use crate::config::Key;

/// What the daemon answers; an `Err` crosses the socket as `error: ...`.
pub type Reply = Result<String, String>;

#[derive(Subcommand, Clone, Debug)]
pub enum Request {
    /// Set a colour (#rrggbb) or the dim factor until the daemon restarts.
    #[command(name = "set")]
    Set { key: Key, value: String },
    /// Print one key, or every key with its current value.
    #[command(name = "get")]
    Get { key: Option<Key> },
    /// Drop the values `set` made, back to the config file and flags.
    #[command(name = "reset")]
    Reset,
    /// Print the keyboard connection, workspaces and media state.
    #[command(name = "status")]
    Status,
    /// Print the key presses of the last DAYS days (default 30) as a heatmap per layer.
    #[command(name = "usage")]
    Usage { days: Option<u64> },
}

impl Request {
    fn line(&self) -> String {
        match self {
            Self::Set { key, value } => format!("set {} {value}", key.name()),
            Self::Get { key: Some(key) } => format!("get {}", key.name()),
            Self::Get { key: None } => "get".to_string(),
            Self::Reset => "reset".to_string(),
            Self::Status => "status".to_string(),
            Self::Usage { .. } => "usage".to_string(),
        }
    }

    fn parse(line: &str) -> Result<Self, String> {
        let key = |name: &str| Key::from_str(name, false);
        let words: Vec<&str> = line.split_whitespace().collect();
        match words[..] {
            ["set", name, value] => Ok(Self::Set {
                key: key(name)?,
                value: value.to_string(),
            }),
            ["get"] => Ok(Self::Get { key: None }),
            ["get", name] => Ok(Self::Get {
                key: Some(key(name)?),
            }),
            ["reset"] => Ok(Self::Reset),
            ["status"] => Ok(Self::Status),
            _ => Err(format!("unknown request {line:?}")),
        }
    }
}

pub fn path() -> Option<PathBuf> {
    dirs::runtime_dir().map(|dir| dir.join("moonlander-leds.sock"))
}

/// Send one request to the daemon and return its reply.
pub fn send(request: &Request) -> anyhow::Result<String> {
    let path = path().ok_or(anyhow!("No XDG_RUNTIME_DIR"))?;
    let mut stream = UnixStream::connect(&path)
        .map_err(|err| anyhow!("Daemon not reachable at {}: {err}", path.display()))?;
    writeln!(stream, "{}", request.line())?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply)?;
    match reply.strip_prefix("error: ") {
        Some(err) => Err(anyhow!("{}", err.trim_end())),
        None => Ok(reply),
    }
}

fn answer(
    stream: UnixStream,
    tx: &Sender<Update>,
) {
    let mut line = String::new();
    let mut reader = BufReader::new(&stream);
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let reply = match Request::parse(line.trim()) {
        Ok(request) => {
            let (tx_reply, rx_reply) = crossbeam_channel::bounded(1);
            let _ = tx.send(Update::Request(request, tx_reply));
            rx_reply
                .recv()
                .unwrap_or_else(|_| Err("daemon stopping".to_string()))
        }
        Err(err) => Err(err),
    };
    let text = match reply {
        Ok(text) => text,
        Err(err) => format!("error: {err}\n"),
    };
    let _ = (&stream).write_all(text.as_bytes());
}

/// Accept requests and hand them to the main loop.
pub fn serve(tx: Sender<Update>) {
    let Some(path) = path() else {
        warn!("no XDG_RUNTIME_DIR, control socket disabled");
        return;
    };
    // A previous run's socket file blocks the bind.
    let _ = std::fs::remove_file(&path);
    let listener = match UnixListener::bind(&path) {
        Ok(listener) => listener,
        Err(err) => {
            warn!("cannot listen on {}: {err}", path.display());
            return;
        }
    };
    for stream in listener.incoming() {
        let Ok(stream) = stream else {
            continue;
        };
        answer(stream, &tx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_line_round_trip() {
        let requests = [
            Request::Set {
                key: Key::MediaPlaying,
                value: "#00ff88".to_string(),
            },
            Request::Get { key: None },
            Request::Get {
                key: Some(Key::Dim),
            },
            Request::Reset,
            Request::Status,
        ];
        for request in requests {
            let line = request.line();
            assert_eq!(Request::parse(&line).unwrap().line(), line);
        }
        assert!(Request::parse("set nope #000000").is_err());
        assert!(Request::parse("frobnicate").is_err());
    }
}
