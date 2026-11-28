//! The device thread: finds the base station, enables ChatMix on it, forwards its reports and
//! reopens it after a replug.

use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::anyhow;
use hidapi::HidApi;
use hidapi::HidDevice;
use novamix::Result;
use novamix::mix::Mix;
use novamix::protocol;

use crate::engine::Event;

/// How long a read waits, so a stop request or a mix to write is seen this quickly.
const READ_TIMEOUT_MS: i32 = 50;

/// How often a missing base station is looked for.
const RETRY: Duration = Duration::from_secs(2);

pub fn spawn(
    tx_event: pipewire::channel::Sender<Event>,
    rx_stop: crossbeam_channel::Receiver<()>,
    rx_dial: crossbeam_channel::Receiver<Mix>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut api = match HidApi::new() {
            Ok(api) => api,
            Err(err) => {
                log::error!("hidapi: {err}");
                return;
            }
        };
        let mut missing_logged = false;
        loop {
            match open(&mut api) {
                Ok(device) => {
                    missing_logged = false;
                    let stopped = serve(&device, &tx_event, &rx_stop, &rx_dial);
                    let _ = tx_event.send(Event::Disconnected);
                    if stopped {
                        return;
                    }
                }
                Err(err) if !missing_logged => {
                    log::warn!("{err}, waiting for it");
                    missing_logged = true;
                }
                Err(_err) => {}
            }
            if rx_stop.recv_timeout(RETRY).is_ok() {
                return;
            }
        }
    })
}

fn open(api: &mut HidApi) -> Result<HidDevice> {
    api.refresh_devices()?;
    let info = api
        .device_list()
        .find(|info| {
            info.vendor_id() == protocol::VID
                && info.product_id() == protocol::PID
                && info.interface_number() == protocol::INTERFACE
        })
        .ok_or(anyhow!("No Arctis Nova Pro Wireless base station"))?;
    log::debug!("opening {:?}", info.path());
    let device = info.open_device(api)?;
    Ok(device)
}

/// Reads until the device goes away (false) or a stop is asked for (true).
fn serve(
    device: &HidDevice,
    tx_event: &pipewire::channel::Sender<Event>,
    rx_stop: &crossbeam_channel::Receiver<()>,
    rx_dial: &crossbeam_channel::Receiver<Mix>,
) -> bool {
    // Mixes set while the base station was away are stale
    while rx_dial.try_recv().is_ok() {}
    if let Err(err) = enable(device, true) {
        log::error!("enabling ChatMix: {err}");
        return false;
    }
    let _ = tx_event.send(Event::Connected);
    let mut message_buf = [0; protocol::MESSAGE_LEN];
    loop {
        if rx_stop.try_recv().is_ok() {
            let _ = enable(device, false);
            return true;
        }
        // Only the newest of several queued mixes matters
        if let Some(mix) = rx_dial.try_iter().last() {
            log::info!("writing {mix} to the base station");
            if let Err(err) = device.write(&protocol::chatmix(mix)) {
                log::error!("writing ChatMix: {err}");
            }
        }
        let len = match device.read_timeout(&mut message_buf, READ_TIMEOUT_MS) {
            Ok(len) => len,
            Err(err) => {
                log::debug!("read: {err}");
                return false;
            }
        };
        log::trace!("{:02x?}", &message_buf[..len]);
        let Some(report) = protocol::decode(&message_buf[..len]) else {
            continue;
        };
        let _ = tx_event.send(Event::Report(report));
    }
}

/// Turns the dial's ChatMix mode and the Sonar icon on or off.
fn enable(
    device: &HidDevice,
    on: bool,
) -> Result<()> {
    device.write(&protocol::sonar_icon(on))?;
    device.write(&protocol::chatmix_enable(on))?;
    Ok(())
}
