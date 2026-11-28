//! The daemon's control socket: one length-prefixed bincode [`Request`] in, one [`Response`] out
//! per connection. A request frame starts with [`VERSION`]; a daemon that sees another first byte
//! answers `Response::Error` (its index never changes, so even a client from before the version
//! byte shows the message).

use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use anyhow::anyhow;

use crate::Result;
use crate::mix::Mix;
use crate::mix::Sink;

/// Bumped whenever a message changes shape. Not a variant index of the first protocol (0..=3).
pub const VERSION: u8 = 0xA1;

/// What a daemon says to a client that sent another version.
pub const VERSION_MISMATCH: &str =
    "client and daemon versions differ; restart the daemon or update the novamix CLI";

/// Larger frames are refused, so a stray client cannot make the daemon allocate gigabytes.
const MAX_FRAME: u32 = 64 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, bincode::Encode, bincode::Decode)]
pub enum Request {
    Status,
    /// Sets both sinks until the dial moves again.
    Set(Mix),
    /// The routing rules and the playback streams alive now.
    Routes,
    /// Moves a playback stream (a node id from `Routes`) to a sink, until it is routed again.
    Move(u32, Sink),
    /// The upstream sink; with a name or node id, retargets the sinks to it.
    Output(Option<String>),
}

#[derive(Clone, Debug, Eq, PartialEq, bincode::Encode, bincode::Decode)]
pub enum Response {
    Status(Status),
    Routes(Routes),
    Ok,
    Error(String),
    Output(Output),
}

/// The real sink the three sinks play to.
#[derive(Clone, Debug, Default, Eq, PartialEq, bincode::Encode, bincode::Decode)]
pub struct Output {
    /// The `node.name`; none while there is no real sink.
    pub name: Option<String>,
    /// Which rule chose it, as the daemon logged it.
    pub how: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, bincode::Encode, bincode::Decode)]
pub struct Status {
    /// The base station is open and ChatMix is enabled on it.
    pub connected: bool,
    /// The sinks that exist, with their PipeWire node ids and `node.name`s.
    pub sinks: Vec<(Sink, u32, String)>,
    pub mix: Mix,
}

/// A routing rule: streams with all of one `matches` entry's properties go to `sink`.
#[derive(Clone, Debug, Eq, PartialEq, bincode::Encode, bincode::Decode)]
pub struct Rule {
    pub sink: Sink,
    pub matches: Vec<Vec<(String, String)>>,
}

/// A playback stream.
#[derive(Clone, Debug, Eq, PartialEq, bincode::Encode, bincode::Decode)]
pub struct Stream {
    pub id: u32,
    /// `application.name`, else `node.name`.
    pub name: String,
    /// `application.process.binary`, empty when unknown.
    pub binary: String,
    /// The `node.name` of the node it plays to, if linked.
    pub target: Option<String>,
    /// The sink of the first rule that matches it; none when no rule does, so it plays
    /// wherever the default sink sends it.
    pub rule: Option<Sink>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, bincode::Encode, bincode::Decode)]
pub struct Routes {
    pub rules: Vec<Rule>,
    pub streams: Vec<Stream>,
}

/// `$XDG_RUNTIME_DIR/novamix.sock`.
pub fn socket_path() -> Result<PathBuf> {
    let dir = dirs::runtime_dir().ok_or(anyhow!("No XDG_RUNTIME_DIR"))?;
    Ok(dir.join("novamix.sock"))
}

pub fn write<T: bincode::Encode>(
    w_stream: &mut impl Write,
    message: &T,
) -> Result<()> {
    let bytes = bincode::encode_to_vec(message, bincode::config::standard())?;
    w_stream.write_all(&(bytes.len() as u32).to_le_bytes())?;
    w_stream.write_all(&bytes)?;
    Ok(())
}

/// A request frame: [`VERSION`], then the bincode message.
pub fn write_request(
    w_stream: &mut impl Write,
    request: &Request,
) -> Result<()> {
    let mut bytes = vec![VERSION];
    bytes.extend(bincode::encode_to_vec(
        request,
        bincode::config::standard(),
    )?);
    w_stream.write_all(&(bytes.len() as u32).to_le_bytes())?;
    w_stream.write_all(&bytes)?;
    Ok(())
}

/// The client's frame was written by another version.
#[derive(Debug)]
pub struct VersionMismatch;
impl std::fmt::Display for VersionMismatch {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.write_str(VERSION_MISMATCH)
    }
}
impl std::error::Error for VersionMismatch {}

pub fn read_request(r_stream: &mut impl Read) -> Result<Request> {
    let frame = read_frame(r_stream)?;
    match frame.split_first() {
        Some((&VERSION, body)) => {
            let (request, _len) = bincode::decode_from_slice(body, bincode::config::standard())?;
            Ok(request)
        }
        _ => Err(VersionMismatch.into()),
    }
}

/// A response, or a clear error when the daemon hung up on a request it could not read
/// (an older daemon does not know the version byte).
pub fn read_response(r_stream: &mut impl Read) -> Result<Response> {
    read(r_stream).map_err(|err| match err.downcast_ref::<std::io::Error>() {
        Some(io) if io.kind() == std::io::ErrorKind::UnexpectedEof => anyhow!(VERSION_MISMATCH),
        _ => err,
    })
}

fn read_frame(r_stream: &mut impl Read) -> Result<Vec<u8>> {
    let mut len_buf = [0; 4];
    r_stream.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf);
    if len > MAX_FRAME {
        return Err(anyhow!("Frame of {len} bytes is too large"));
    }
    let mut message_buf = vec![0; len as usize];
    r_stream.read_exact(&mut message_buf)?;
    Ok(message_buf)
}

pub fn read<T: bincode::Decode<()>>(r_stream: &mut impl Read) -> Result<T> {
    let (message, _len) =
        bincode::decode_from_slice(&read_frame(r_stream)?, bincode::config::standard())?;
    Ok(message)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_round_trip() {
        let mut buf = Vec::new();
        write_request(&mut buf, &Request::Set(Mix::new(40, 90))).unwrap();
        let request = read_request(&mut buf.as_slice()).unwrap();
        assert!(request == Request::Set(Mix::new(40, 90)));
    }

    #[test]
    fn test_old_client_is_a_version_mismatch() {
        // A frame from before the version byte: the bincode of Request::Status alone
        let mut buf = Vec::new();
        write(&mut buf, &Request::Status).unwrap();
        let err = read_request(&mut buf.as_slice()).unwrap_err();
        assert!(err.downcast_ref::<VersionMismatch>().is_some());
    }

    #[test]
    fn test_hangup_is_a_version_mismatch() {
        let err = read_response(&mut [].as_slice()).unwrap_err();
        assert!(err.to_string() == VERSION_MISMATCH);
    }

    #[test]
    fn test_rejects_huge_frame() {
        let buf = u32::MAX.to_le_bytes();
        assert!(read_request(&mut buf.as_slice()).is_err());
    }
}
