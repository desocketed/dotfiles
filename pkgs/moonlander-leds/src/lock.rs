//! Whether the desktop is locked: niri has no lock state, so look for the
//! running `swaylock` (the locker the Super+Alt+L bind spawns).

use std::time::Duration;

use crossbeam_channel::Sender;

use crate::Update;

const POLL: Duration = Duration::from_millis(500);

/// A process list entry is swaylock when its `comm` file says so.
fn is_swaylock(comm: &str) -> bool {
    comm.trim_end() == "swaylock"
}

fn locked() -> bool {
    let Ok(procs) = std::fs::read_dir("/proc") else {
        return false;
    };
    procs
        .flatten()
        .filter_map(|entry| std::fs::read_to_string(entry.path().join("comm")).ok())
        .any(|comm| is_swaylock(&comm))
}

pub fn source(tx: Sender<Update>) {
    let mut last = None;
    loop {
        let now = locked();
        if last != Some(now) {
            last = Some(now);
            if tx.send(Update::Locked(now)).is_err() {
                return;
            }
        }
        std::thread::sleep(POLL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_swaylock() {
        assert!(is_swaylock("swaylock\n"));
        assert!(!is_swaylock("swaylock-extra\n"));
        assert!(!is_swaylock("niri\n"));
    }
}
