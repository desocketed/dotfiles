//! The control socket thread: one request per connection, answered by the main loop.

use std::os::unix::net::UnixListener;
use std::os::unix::net::UnixStream;
use std::path::Path;

use novamix::Result;
use novamix::ipc;
use novamix::ipc::Request;
use novamix::ipc::Response;

use crate::engine::Event;

pub fn spawn(
    path: &Path,
    tx_event: pipewire::channel::Sender<Event>,
) -> Result<()> {
    // A socket left by a daemon that died; a live one would answer
    if path.exists() && UnixStream::connect(path).is_err() {
        std::fs::remove_file(path)?;
    }
    let listener = UnixListener::bind(path)?;
    log::info!("control socket {}", path.display());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                continue;
            };
            let request: Request = match ipc::read_request(&mut stream) {
                Ok(request) => request,
                Err(err) if err.downcast_ref::<ipc::VersionMismatch>().is_some() => {
                    log::warn!("request from a client of another version");
                    let _ = ipc::write(&mut stream, &Response::Error(ipc::VERSION_MISMATCH.into()));
                    continue;
                }
                Err(err) => {
                    log::warn!("bad request: {err}");
                    continue;
                }
            };
            let (tx_reply, rx_reply) = crossbeam_channel::bounded(1);
            if tx_event.send(Event::Request(request, tx_reply)).is_err() {
                return;
            }
            let response = rx_reply
                .recv()
                .unwrap_or(Response::Error("Daemon is stopping".to_string()));
            let _ = ipc::write(&mut stream, &response);
        }
    });
    Ok(())
}
