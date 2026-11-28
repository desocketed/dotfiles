mod command;
mod show;

use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::anyhow;
use clap::Parser;
use novamix::Result;
use novamix::ipc;
use novamix::ipc::Request;
use novamix::ipc::Response;
use novamix::ipc::Stream;

use crate::command::Command;

/// Controls the running novamix-daemon: `novamix help` lists the objects, `novamix OBJECT help`
/// each one's commands.
#[derive(Parser)]
#[command(
    name = "novamix",
    version,
    disable_help_flag = true,
    disable_help_subcommand = true
)]
struct Args {
    /// The daemon's socket; default: $XDG_RUNTIME_DIR/novamix.sock.
    #[arg(long, env = "NOVAMIX_SOCKET")]
    socket: Option<PathBuf>,

    /// OBJECT [ ARGS ]; see `novamix help`.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    command: Vec<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("novamix: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    let command = command::parse(&args.command)?;
    if let Command::Help(text) = command {
        print!("{text}");
        return Ok(());
    }
    let path = match args.socket {
        Some(path) => path,
        None => ipc::socket_path()?,
    };
    print!("{}", execute(command, &Daemon(path))?);
    Ok(())
}

/// The daemon's socket; it answers one request per connection.
struct Daemon(PathBuf);
impl Daemon {
    fn ask(
        &self,
        request: Request,
    ) -> Result<Response> {
        let mut stream = UnixStream::connect(&self.0)
            .map_err(|err| anyhow!("Cannot reach the daemon at {}: {err}", self.0.display()))?;
        ipc::write_request(&mut stream, &request)?;
        match ipc::read_response(&mut stream)? {
            Response::Error(message) => Err(anyhow!(message)),
            response => Ok(response),
        }
    }

    fn status(&self) -> Result<ipc::Status> {
        match self.ask(Request::Status)? {
            Response::Status(status) => Ok(status),
            _ => Err(anyhow!("Unexpected answer from the daemon")),
        }
    }

    fn routes(&self) -> Result<ipc::Routes> {
        match self.ask(Request::Routes)? {
            Response::Routes(routes) => Ok(routes),
            _ => Err(anyhow!("Unexpected answer from the daemon")),
        }
    }

    fn output(
        &self,
        wanted: Option<String>,
    ) -> Result<ipc::Output> {
        match self.ask(Request::Output(wanted))? {
            Response::Output(output) => Ok(output),
            _ => Err(anyhow!("Unexpected answer from the daemon")),
        }
    }

    fn command(
        &self,
        request: Request,
    ) -> Result<()> {
        match self.ask(request)? {
            Response::Ok => Ok(()),
            _ => Err(anyhow!("Unexpected answer from the daemon")),
        }
    }
}

/// Asks the daemon and returns what to print.
fn execute(
    command: Command,
    daemon: &Daemon,
) -> Result<String> {
    match command {
        Command::Help(text) => Ok(text.to_string()),
        Command::Status => Ok(show::status(&daemon.status()?)),
        Command::ShowMix => Ok(format!("{}\n", daemon.status()?.mix)),
        Command::SetMix(mix) => {
            daemon.command(Request::Set(mix))?;
            Ok(format!("{mix}\n"))
        }
        Command::Output(wanted) => Ok(show::output(&daemon.output(wanted)?)),
        Command::Routes => Ok(show::routes(&daemon.routes()?)),
        Command::Move(wanted, sink) => {
            let routes = daemon.routes()?;
            let stream = find(&routes.streams, &wanted)?;
            daemon.command(Request::Move(stream.id, sink))?;
            Ok(format!("{} ({}) -> {sink}\n", stream.name, stream.id))
        }
    }
}

/// A node id, or the one stream whose name or binary holds `wanted`.
fn find<'a>(
    streams: &'a [Stream],
    wanted: &str,
) -> Result<&'a Stream> {
    if let Ok(id) = wanted.parse::<u32>() {
        return streams
            .iter()
            .find(|stream| stream.id == id)
            .ok_or(anyhow!("No playback stream with node id {id}"));
    }
    let wanted = wanted.to_lowercase();
    let found: Vec<&Stream> = streams
        .iter()
        .filter(|stream| {
            stream.name.to_lowercase().contains(&wanted)
                || stream.binary.to_lowercase().contains(&wanted)
        })
        .collect();
    match found.as_slice() {
        [stream] => Ok(stream),
        [] => Err(anyhow!(
            "No playback stream matches {wanted}; see: route list"
        )),
        many => {
            let ids: Vec<String> = many
                .iter()
                .map(|stream| format!("{} ({})", stream.name, stream.id))
                .collect();
            Err(anyhow!(
                "{wanted} matches {}; use a node id",
                ids.join(", ")
            ))
        }
    }
}

#[cfg(test)]
mod test {
    use std::os::unix::net::UnixListener;

    use novamix::mix::Mix;
    use novamix::mix::Sink;

    use super::*;

    fn stream(
        id: u32,
        name: &str,
        target: &str,
    ) -> Stream {
        Stream {
            id,
            name: name.into(),
            binary: name.to_lowercase(),
            target: Some(target.into()),
            rule: None,
        }
    }

    /// Answers `requests.len()` connections like the daemon, returning what it was asked.
    fn fake_daemon(
        dir: &std::path::Path,
        answers: Vec<Response>,
    ) -> (Daemon, std::thread::JoinHandle<Vec<Request>>) {
        let path = dir.join("novamix.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let thread = std::thread::spawn(move || {
            let mut asked = Vec::new();
            for answer in answers {
                let (mut stream, _addr) = listener.accept().unwrap();
                asked.push(ipc::read_request(&mut stream).unwrap());
                ipc::write(&mut stream, &answer).unwrap();
            }
            asked
        });
        (Daemon(path), thread)
    }

    fn dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("novamix-cli-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(format!("{:?}", std::thread::current().id()).replace(['(', ')'], ""))
    }

    #[test]
    fn test_status() {
        let dir = dir();
        std::fs::create_dir_all(&dir).unwrap();
        let status = ipc::Status {
            connected: true,
            sinks: vec![
                (Sink::Game, 41, "input.GAME".into()),
                (Sink::Chat, 42, "input.CHAT".into()),
                (Sink::Global, 43, "input.GLOBAL".into()),
            ],
            mix: Mix::new(70, 100),
        };
        let (daemon, thread) = fake_daemon(&dir, vec![Response::Status(status)]);
        let out = execute(Command::Status, &daemon).unwrap();
        assert!(thread.join().unwrap() == [Request::Status]);
        assert!(out.contains("connected"));
        assert!(out.contains("game 70% chat 100%"));
        assert!(out.contains("input.GAME node 41"));
        assert!(out.contains("input.GLOBAL node 43"));
        assert!(out.contains("node 42"));
    }

    #[test]
    fn test_move_by_name() {
        let dir = dir();
        std::fs::create_dir_all(&dir).unwrap();
        let routes = ipc::Routes {
            rules: Vec::new(),
            streams: vec![
                stream(90, "Firefox", "headset"),
                stream(91, "mpv", "headset"),
            ],
        };
        let (daemon, thread) = fake_daemon(&dir, vec![Response::Routes(routes), Response::Ok]);
        let out = execute(Command::Move("fire".into(), Sink::Chat), &daemon).unwrap();
        assert!(thread.join().unwrap()[1] == Request::Move(90, Sink::Chat));
        assert!(out == "Firefox (90) -> chat\n");
    }

    #[test]
    fn test_find() {
        let streams = [stream(1, "Firefox", "x"), stream(2, "Firefox Nightly", "x")];
        assert!(find(&streams, "2").unwrap().id == 2);
        assert!(find(&streams, "nightly").unwrap().id == 2);
        assert!(find(&streams, "firefox").is_err());
        assert!(find(&streams, "mpv").is_err());
    }

    #[test]
    fn test_no_daemon() {
        let err =
            execute(Command::Status, &Daemon("/nonexistent/novamix.sock".into())).unwrap_err();
        assert!(err.to_string().contains("Cannot reach the daemon"));
    }
}
