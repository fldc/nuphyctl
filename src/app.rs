use crate::cli::{Cli, Command, DeviceSelector, RawSubcommand, RgbSubcommand};
use crate::color::{parse_hex_bytes, RgbColor};
use crate::hid_transport::{
    clear_input_reports, list_devices, open_selected_device, send_report, REPORT_LEN,
};
use crate::nuphy_protocol::{KeyboardProtocol, SessionKey};
use anyhow::{bail, Result};
use hidapi::HidApi;
use std::{thread, time::Duration};

const RGB_SET_MAX_ATTEMPTS: usize = 4;
const RGB_SET_RETRY_BASE_DELAY_MS: u64 = 120;

const RETRYABLE_RGB_ERROR_PATTERNS: [&str; 6] = [
    "no such device",
    "protocol error",
    "timeout waiting for hid response",
    "hid read_timeout failed",
    "no matching hid device found",
    "short write",
];

pub fn run(cli: Cli, api: &HidApi) -> Result<()> {
    match cli.command {
        Command::List => {
            list_devices(api);
            Ok(())
        }
        Command::Rgb(rgb) => match rgb.action {
            RgbSubcommand::Set(args) => run_rgb_set(api, &args.device, &args.hex, args.brightness),
        },
        Command::Raw(raw) => match raw.action {
            RawSubcommand::Send(args) => run_raw_send(api, &args.device, args.report_id, &args.hex),
        },
    }
}

fn run_rgb_set(
    api: &HidApi,
    selector: &DeviceSelector,
    color_hex: &str,
    brightness: u8,
) -> Result<()> {
    let (color, normalized_hex) = RgbColor::from_hex(color_hex)?;

    for attempt in 1..=RGB_SET_MAX_ATTEMPTS {
        match run_rgb_set_once(api, selector, color, brightness) {
            Ok(session_key) => {
                if attempt == 1 {
                    println!(
                        "sent static color {} (rgb={}, {}, {} brightness={} key=0x{:02x})",
                        normalized_hex,
                        color.r,
                        color.g,
                        color.b,
                        brightness,
                        session_key.value(),
                    );
                } else {
                    println!(
                        "sent static color {} (rgb={}, {}, {} brightness={} key=0x{:02x}) after retry {}",
                        normalized_hex,
                        color.r,
                        color.g,
                        color.b,
                        brightness,
                        session_key.value(),
                        attempt,
                    );
                }
                return Ok(());
            }
            Err(err) => {
                if !is_retryable_rgb_error(&err) || attempt == RGB_SET_MAX_ATTEMPTS {
                    return Err(err);
                }

                eprintln!(
                    "transient HID error (attempt {}/{}): {} -- retrying",
                    attempt, RGB_SET_MAX_ATTEMPTS, err
                );
                thread::sleep(Duration::from_millis(
                    RGB_SET_RETRY_BASE_DELAY_MS * attempt as u64,
                ));
            }
        }
    }

    bail!("unexpected RGB retry loop exit")
}

fn run_rgb_set_once(
    api: &HidApi,
    selector: &DeviceSelector,
    color: RgbColor,
    brightness: u8,
) -> Result<SessionKey> {
    let dev = open_selected_device(api, selector)?;
    clear_input_reports(&dev)?;

    let protocol = KeyboardProtocol::new(&dev)?;
    protocol.set_static_rgb(color, brightness)?;

    Ok(protocol.session_key())
}

fn run_raw_send(api: &HidApi, selector: &DeviceSelector, report_id: u8, hex: &str) -> Result<()> {
    let bytes = parse_hex_bytes(hex)?;
    if bytes.len() != REPORT_LEN {
        bail!(
            "raw report must be exactly {} bytes, got {}",
            REPORT_LEN,
            bytes.len()
        );
    }

    let dev = open_selected_device(api, selector)?;
    send_report(&dev, report_id, &bytes)?;

    println!("sent raw report_id={} bytes={}", report_id, bytes.len());
    Ok(())
}

fn is_retryable_rgb_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        let msg = cause.to_string().to_ascii_lowercase();
        RETRYABLE_RGB_ERROR_PATTERNS
            .iter()
            .any(|pattern| msg.contains(pattern))
    })
}
