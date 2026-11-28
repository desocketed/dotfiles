//! The command line as keywords, iproute2 style: `route list`, `route firefox chat`,
//! `mix balance -30`. `help` works before or after an object: `help route`, `route help`.

use anyhow::anyhow;
use novamix::Result;
use novamix::mix::Mix;
use novamix::mix::Sink;

pub const HELP: &str = "\
Usage: novamix [ --socket PATH ] OBJECT [ ARGS ]
       novamix help [ OBJECT ]

OBJECT := { status | mix | route | output | help }

Talks to the running novamix-daemon over its socket, $XDG_RUNTIME_DIR/novamix.sock.
";

pub const HELP_STATUS: &str = "\
Usage: novamix status

Shows whether the base station is connected, the mix and the sinks with their node ids.
";

pub const HELP_MIX: &str = "\
Usage: novamix mix
       novamix mix GAME [ CHAT ]
       novamix mix balance BALANCE      (or: mix - BALANCE)
       novamix mix reset

GAME, CHAT := 0..100 (percent, a trailing % is fine)
BALANCE    := -100..100, as the dial: -100 is all game, 0 the centre, 100 all chat

mix            shows the mix
mix 70         game 70%, chat 100%
mix 70 40      game 70%, chat 40%
mix balance 30 game 70%, chat 100%: 30 towards chat
mix reset      both at 100%
A mix set here holds until the dial moves.
";

pub const HELP_ROUTE: &str = "\
Usage: novamix route [ list ]
       novamix route STREAM { game | chat | global }

STREAM := a node id from `route list`, or part of an application name or binary
          (case does not matter); it must match one stream

route list     shows the rules and each playing stream's sink; (no rule) marks a stream
               no rule matched
route STREAM chat   moves the stream to CHAT, until it is routed again
GLOBAL is the sink the dial never changes; unmatched streams stay on it
";

pub const HELP_OUTPUT: &str = "\
Usage: novamix output
       novamix output SINK

SINK := a node.name or node id of a real sink (see `wpctl status`)

output        shows the sink GAME, CHAT and GLOBAL play to, and how it was chosen
output SINK   plays them to SINK now and remembers it for the next start
The sinks of novamix itself are refused.
";

#[derive(Debug, Eq, PartialEq)]
pub enum Command {
    Help(&'static str),
    Status,
    ShowMix,
    SetMix(Mix),
    Routes,
    Move(String, Sink),
    Output(Option<String>),
}

/// What follows the global options.
pub fn parse(words: &[String]) -> Result<Command> {
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    match words.as_slice() {
        [] | ["help"] => Ok(Command::Help(HELP)),
        ["help", object] => Ok(Command::Help(help_for(object)?)),
        ["status"] => Ok(Command::Status),
        ["status", "help"] => Ok(Command::Help(HELP_STATUS)),
        ["mix"] => Ok(Command::ShowMix),
        ["mix", "help"] => Ok(Command::Help(HELP_MIX)),
        ["mix", "reset"] => Ok(Command::SetMix(Mix::full())),
        ["mix", "balance" | "-", balance] => Ok(Command::SetMix(balanced(number(balance)?)?)),
        ["mix", game] => Ok(Command::SetMix(Mix::new(percent(game)?, 100))),
        ["mix", game, chat] => Ok(Command::SetMix(Mix::new(percent(game)?, percent(chat)?))),
        ["route"] | ["route", "list"] => Ok(Command::Routes),
        ["route", "help"] => Ok(Command::Help(HELP_ROUTE)),
        ["route", stream, sink] => Ok(Command::Move((*stream).to_string(), sink.parse()?)),
        ["output"] => Ok(Command::Output(None)),
        ["output", "help"] => Ok(Command::Help(HELP_OUTPUT)),
        ["output", sink] => Ok(Command::Output(Some((*sink).to_string()))),
        [object, ..] if is_object(object) => Err(anyhow!("Too many arguments; try: {object} help")),
        [object, ..] => Err(anyhow!("Unknown object {object}; try: help")),
    }
}

fn is_object(word: &str) -> bool {
    matches!(word, "status" | "mix" | "route" | "output")
}

fn help_for(object: &str) -> Result<&'static str> {
    match object {
        "status" => Ok(HELP_STATUS),
        "mix" => Ok(HELP_MIX),
        "route" => Ok(HELP_ROUTE),
        "output" => Ok(HELP_OUTPUT),
        _ => Err(anyhow!("Unknown object {object}; try: help")),
    }
}

fn number(text: &str) -> Result<i32> {
    text.trim_end_matches('%')
        .parse()
        .map_err(|_err| anyhow!("{text} is not a number"))
}

fn percent(text: &str) -> Result<u8> {
    match number(text)? {
        level @ 0..=100 => Ok(level as u8),
        _ => Err(anyhow!("{text} is outside 0..100")),
    }
}

/// The dial: towards chat lowers game, towards game lowers chat.
fn balanced(balance: i32) -> Result<Mix> {
    if !(-100..=100).contains(&balance) {
        return Err(anyhow!("Balance {balance} is outside -100..100"));
    }
    let level = 100 - balance.unsigned_abs() as u8;
    Ok(if balance > 0 {
        Mix::new(level, 100)
    } else {
        Mix::new(100, level)
    })
}

#[cfg(test)]
mod test {
    use super::*;

    fn parse_line(line: &str) -> Result<Command> {
        parse(
            &line
                .split_whitespace()
                .map(String::from)
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn test_help_both_orders() {
        assert!(parse_line("help route").unwrap() == Command::Help(HELP_ROUTE));
        assert!(parse_line("route help").unwrap() == Command::Help(HELP_ROUTE));
        assert!(parse_line("mix help").unwrap() == Command::Help(HELP_MIX));
        assert!(parse_line("help").unwrap() == Command::Help(HELP));
        assert!(parse_line("").unwrap() == Command::Help(HELP));
    }

    #[test]
    fn test_mix() {
        assert!(parse_line("mix").unwrap() == Command::ShowMix);
        assert!(parse_line("mix 70%").unwrap() == Command::SetMix(Mix::new(70, 100)));
        assert!(parse_line("mix 70 40").unwrap() == Command::SetMix(Mix::new(70, 40)));
        assert!(parse_line("mix reset").unwrap() == Command::SetMix(Mix::full()));
        assert!(parse_line("mix balance 30").unwrap() == Command::SetMix(Mix::new(70, 100)));
        assert!(parse_line("mix balance -30").unwrap() == Command::SetMix(Mix::new(100, 70)));
        assert!(parse_line("mix balance 0").unwrap() == Command::SetMix(Mix::full()));
        assert!(parse_line("mix - -30").unwrap() == Command::SetMix(Mix::new(100, 70)));
        assert!(parse_line("mix 101").is_err());
        assert!(parse_line("mix balance 101").is_err());
        assert!(parse_line("mix loud").is_err());
    }

    #[test]
    fn test_output() {
        assert!(parse_line("output").unwrap() == Command::Output(None));
        assert!(
            parse_line("output alsa_output.usb").unwrap()
                == Command::Output(Some("alsa_output.usb".into()))
        );
        assert!(parse_line("output help").unwrap() == Command::Help(HELP_OUTPUT));
        assert!(parse_line("output a b").is_err());
    }

    #[test]
    fn test_route() {
        assert!(parse_line("route list").unwrap() == Command::Routes);
        assert!(parse_line("route").unwrap() == Command::Routes);
        assert!(
            parse_line("route Firefox chat").unwrap()
                == Command::Move("Firefox".into(), Sink::Chat)
        );
        assert!(parse_line("route 57 GAME").unwrap() == Command::Move("57".into(), Sink::Game));
        assert!(
            parse_line("route firefox global").unwrap()
                == Command::Move("firefox".into(), Sink::Global)
        );
        assert!(parse_line("route firefox music").is_err());
        assert!(parse_line("status now").is_err());
        assert!(parse_line("volume").is_err());
    }
}
