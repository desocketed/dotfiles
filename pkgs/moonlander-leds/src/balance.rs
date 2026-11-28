//! The ChatMix balance, through the `novamix` command line: polled to show on
//! the keyboard, and set when a key of the strip is pressed.

use std::process::Command;
use std::time::Duration;

use crossbeam_channel::Receiver;
use crossbeam_channel::RecvTimeoutError;
use crossbeam_channel::Sender;
use log::warn;

use crate::Update;

const POLL: Duration = Duration::from_millis(500);

/// The balance in `novamix mix`'s `game 70% chat 100%`: towards chat is
/// positive, so 70/100 is 30 and 100/60 is -40.
pub fn parse(text: &str) -> Option<i32> {
    let levels: Vec<i32> = text
        .split_whitespace()
        .filter_map(|word| word.strip_suffix('%')?.parse().ok())
        .collect();
    match levels[..] {
        [game, chat] => Some(chat - game),
        _ => None,
    }
}

fn novamix(args: &[&str]) -> Option<String> {
    let output = Command::new("novamix")
        .args(args)
        .output()
        .inspect_err(|err| warn!("cannot run novamix: {err}"))
        .ok()?;
    if !output.status.success() {
        warn!(
            "novamix {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Applies the balances the keyboard asks for and reports the current one
/// whenever it changes, whoever changed it (the base station's dial too).
pub fn source(
    sets: Receiver<i32>,
    tx: Sender<Update>,
) {
    let mut last = None;
    loop {
        match sets.recv_timeout(POLL) {
            Ok(balance) => {
                novamix(&["mix", "balance", &balance.to_string()]);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let balance = novamix(&["mix"]).and_then(|text| parse(&text));
        if balance != last {
            last = balance;
            if tx.send(Update::Balance(balance)).is_err() {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        assert_eq!(parse("game 100% chat 100%\n"), Some(0));
        assert_eq!(parse("game 70% chat 100%\n"), Some(30));
        assert_eq!(parse("game 100% chat 60%\n"), Some(-40));
        assert_eq!(parse("not running"), None);
    }
}
