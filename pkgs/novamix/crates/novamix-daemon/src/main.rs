mod engine;
mod hid;
mod names;
mod pw;
mod route;
mod socket;
mod state;

use std::path::PathBuf;

use anyhow::anyhow;
use clap::Parser;
use novamix::Result;
use pipewire::context::ContextRc;
use pipewire::loop_::Signal;
use pipewire::main_loop::MainLoopRc;

use crate::engine::Engine;
use crate::names::Names;
use crate::pw::PwAudio;

/// ChatMix for the Arctis Nova Pro Wireless: the base station's dial balances the GAME
/// and CHAT sinks.
#[derive(Parser)]
#[command(name = "novamix-daemon", version)]
struct Args {
    /// The sink all three play to, pinned: a node.name. Default: the default sink at start
    /// (else the highest-priority real sink), picked again when it goes away.
    #[arg(long, env = "NOVAMIX_OUTPUT")]
    output: Option<String>,

    /// The control socket; default: $XDG_RUNTIME_DIR/novamix.sock.
    #[arg(long, env = "NOVAMIX_SOCKET")]
    socket: Option<PathBuf>,

    /// Stream routing rules; default: $XDG_CONFIG_HOME/novamix/config.toml.
    #[arg(long, env = "NOVAMIX_CONFIG")]
    config: Option<PathBuf>,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    let socket_path = match args.socket {
        Some(path) => path,
        None => novamix::ipc::socket_path()?,
    };

    // Before the sinks exist, so WirePlumber is up again with the rules when they appear
    let config = route::read(args.config.as_deref())?;
    let rules = route::load(&config)?;
    let write_dial = route::write_dial(&config)?;
    let take_default = route::take_default(&config)?;
    let names = Names::parse(&config)?;
    if let Err(err) = route::install(&rules, &names) {
        log::error!("stream routing: {err}");
    }

    pipewire::init();
    let main_loop = MainLoopRc::new(None).map_err(|err| anyhow!("PipeWire main loop: {err}"))?;
    let context =
        ContextRc::new(&main_loop, None).map_err(|err| anyhow!("PipeWire context: {err}"))?;
    let core = context
        .connect_rc(None)
        .map_err(|err| anyhow!("Cannot connect to PipeWire: {err}"))?;
    let registry = core
        .get_registry_rc()
        .map_err(|err| anyhow!("PipeWire registry: {err}"))?;

    // Losing PipeWire ends the daemon; its supervisor starts it again
    let main_loop_ = main_loop.clone();
    let _core_listener = core
        .add_listener_local()
        .error(move |id, _seq, _res, message| {
            if id == pipewire::core::PW_ID_CORE {
                log::error!("PipeWire: {message}");
                main_loop_.quit();
            }
        })
        .register();

    // Before any thread or child starts, so they inherit the blocked mask and the main loop
    // alone sees these
    let main_loop_ = main_loop.clone();
    let _sigint = main_loop
        .loop_()
        .add_signal_local(Signal::INT, move || main_loop_.quit());
    let main_loop_ = main_loop.clone();
    let _sigterm = main_loop
        .loop_()
        .add_signal_local(Signal::TERM, move || main_loop_.quit());

    let rules: Vec<novamix::ipc::Rule> = rules.iter().map(Into::into).collect();
    let audio = PwAudio::new(args.output, names)?
        .with_rules(rules.clone())
        .with_take_default(take_default);
    let _registry_listener = audio.listen(&registry);
    // Its reply says the registry has listed everything that was there at start
    let audio_ = audio.clone();
    let _settle_listener = core
        .add_listener_local()
        .done(move |_id, _seq| audio_.settle())
        .register();
    core.sync(0)
        .map_err(|err| anyhow!("PipeWire sync: {err}"))?;
    // attach() takes an Fn
    let (tx_dial, rx_dial) = crossbeam_channel::unbounded();
    let mut engine = Engine::new(audio.clone()).with_rules(rules);
    if write_dial {
        engine = engine.with_dial(tx_dial);
    }
    let engine = std::cell::RefCell::new(engine);
    let (tx_event, rx_event) = pipewire::channel::channel();
    let _rx_event = rx_event.attach(main_loop.loop_(), move |event| {
        engine.borrow_mut().handle(event)
    });

    socket::spawn(&socket_path, tx_event.clone())?;
    let (tx_stop, rx_stop) = crossbeam_channel::bounded(1);
    let device_thread = hid::spawn(tx_event, rx_stop, rx_dial);

    main_loop.run();

    // The device thread turns ChatMix off on the base station before it ends
    log::info!("stopping");
    let _ = tx_stop.send(());
    let _ = device_thread.join();
    audio.close();
    let _ = std::fs::remove_file(&socket_path);
    Ok(())
}
