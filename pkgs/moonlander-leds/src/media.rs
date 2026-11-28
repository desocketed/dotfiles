//! The desktop's media player over MPRIS, polled: the active player's playback
//! status and whether it can go back or forward.

use std::time::Duration;

use crossbeam_channel::Sender;
use log::warn;
use mpris::PlaybackStatus;
use mpris::Player;
use mpris::PlayerFinder;

use crate::Update;
use crate::protocol::Media;
use crate::protocol::Playback;

const POLL: Duration = Duration::from_millis(500);

/// playerctld, when running, proxies the most recently active player.
fn find(finder: &PlayerFinder) -> Option<Player> {
    finder
        .iter_players()
        .ok()?
        .flatten()
        .find(|player| player.bus_name_player_name_part() == "playerctld")
        .or_else(|| finder.find_active().ok())
}

fn read(finder: &PlayerFinder) -> Media {
    let Some(player) = find(finder) else {
        return Media::default();
    };
    let playback = match player.get_playback_status() {
        Ok(PlaybackStatus::Playing) => Playback::Playing,
        Ok(PlaybackStatus::Paused | PlaybackStatus::Stopped) => Playback::Paused,
        Err(_) => return Media::default(),
    };
    Media {
        playback,
        can_previous: player.can_go_previous().unwrap_or(false),
        can_next: player.can_go_next().unwrap_or(false),
    }
}

pub fn source(tx: Sender<Update>) {
    loop {
        match PlayerFinder::new() {
            Ok(finder) => {
                let mut last = None;
                loop {
                    let media = read(&finder);
                    if last != Some(media) {
                        last = Some(media);
                        if tx.send(Update::Media(media)).is_err() {
                            return;
                        }
                    }
                    std::thread::sleep(POLL);
                }
            }
            Err(err) => warn!("cannot reach the session bus: {err}"),
        }
        std::thread::sleep(crate::RETRY);
    }
}
