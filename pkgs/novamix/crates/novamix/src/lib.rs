//! The pieces the daemon and its clients share: the base station's HID protocol, the mix and
//! the control socket's messages.

pub mod ipc;
pub mod mix;
pub mod protocol;

pub type Result<T> = anyhow::Result<T>;
