//! Where reports go: the Moonlander's raw HID interface, or stdout.

use hidapi::HidApi;
use hidapi::HidDevice;
use log::debug;
use log::info;
use log::warn;

use crate::protocol::Report;

const ZSA_VENDOR_ID: u16 = 0x3297;
const RAW_HID_USAGE_PAGE: u16 = 0xFF60;
const RAW_HID_USAGE: u16 = 0x61;

pub enum Output {
    Hid {
        api: HidApi,
        device: Option<HidDevice>,
    },
    Print,
}

impl Output {
    pub fn hid() -> anyhow::Result<Self> {
        Ok(Self::Hid {
            api: HidApi::new()?,
            device: None,
        })
    }

    pub fn is_connected(&self) -> bool {
        match self {
            Self::Hid { device, .. } => device.is_some(),
            Self::Print => true,
        }
    }

    /// Try to open the keyboard; true when this call connected it.
    pub fn connect(&mut self) -> bool {
        let Self::Hid { api, device } = self else {
            return false;
        };
        if device.is_some() || api.refresh_devices().is_err() {
            return false;
        }
        let Some(info) = api.device_list().find(|info| {
            info.vendor_id() == ZSA_VENDOR_ID
                && info.usage_page() == RAW_HID_USAGE_PAGE
                && info.usage() == RAW_HID_USAGE
        }) else {
            return false;
        };
        let Ok(opened) = info.open_device(api) else {
            warn!("Moonlander found but not openable (udev permissions?)");
            return false;
        };
        info!("connected to {:?}", info.path());
        *device = Some(opened);
        true
    }

    /// A packet the keyboard sent, if one is waiting.
    pub fn read(&mut self) -> Option<Report> {
        let Self::Hid { device, .. } = self else {
            return None;
        };
        let dev = device.as_ref()?;
        let mut report = [0; crate::protocol::REPORT_LEN];
        match dev.read_timeout(&mut report, 0) {
            Ok(0) => None,
            Ok(_) => Some(report),
            Err(err) => {
                warn!("read failed, will reconnect: {err}");
                *device = None;
                None
            }
        }
    }

    pub fn send(
        &mut self,
        report: &Report,
    ) {
        match self {
            Self::Print => {
                let hex: Vec<String> = report.iter().map(|b| format!("{b:02x}")).collect();
                println!("{}", hex.join(" "));
            }
            Self::Hid { device, .. } => {
                let Some(dev) = device else {
                    return;
                };
                // hidapi wants the report id (0: none) in front.
                let mut buffer = [0; 1 + crate::protocol::REPORT_LEN];
                buffer[1..].copy_from_slice(report);
                match dev.write(&buffer) {
                    Ok(written) => debug!("sent {written} bytes: {report:02x?}"),
                    Err(err) => {
                        warn!("write failed, will reconnect: {err}");
                        *device = None;
                    }
                }
            }
        }
    }
}
