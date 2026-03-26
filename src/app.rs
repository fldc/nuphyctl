use crate::cli::{
    Cli, Command, DeviceSelector, RawSubcommand, RgbColorMode, RgbDirection, RgbEffect,
    RgbSideEffect, RgbSubcommand,
};
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
        Command::Commands => {
            print_commands();
            Ok(())
        }
        Command::List => {
            list_devices(api);
            Ok(())
        }
        Command::Rgb(rgb) => match rgb.action {
            RgbSubcommand::Set(args) => run_rgb_set(
                api,
                &args.device,
                args.effect,
                &args.hex,
                args.brightness,
                args.speed,
                args.direction,
                args.color_mode,
                args.palette_index,
            ),
            RgbSubcommand::Side(args) => run_rgb_side(
                api,
                &args.device,
                args.effect,
                args.hex.as_deref(),
                args.brightness,
                args.speed,
                args.color_mode,
                args.palette_index,
            ),
            RgbSubcommand::Decorative(args) => run_rgb_decorative(
                api,
                &args.device,
                args.effect,
                args.hex.as_deref(),
                args.brightness,
                args.speed,
                args.color_mode,
                args.palette_index,
                args.base_offset,
            ),
        },
        Command::Raw(raw) => match raw.action {
            RawSubcommand::Send(args) => run_raw_send(api, &args.device, args.report_id, &args.hex),
        },
    }
}

fn run_rgb_set(
    api: &HidApi,
    selector: &DeviceSelector,
    effect: RgbEffect,
    color_hex: &str,
    brightness: u8,
    speed: u8,
    direction: RgbDirection,
    color_mode: RgbColorMode,
    palette_index: u8,
) -> Result<()> {
    let (color, normalized_hex) = RgbColor::from_hex(color_hex)?;

    for attempt in 1..=RGB_SET_MAX_ATTEMPTS {
        match run_rgb_set_once(
            api,
            selector,
            effect,
            color,
            brightness,
            speed,
            direction,
            color_mode,
            palette_index,
        ) {
            Ok(session_key) => {
                if attempt == 1 {
                    println!(
                        "sent backlight {} effect color={} brightness={} speed={} direction={} mode={} palette={} key=0x{:02x}",
                        effect.display_name(),
                        normalized_hex,
                        brightness,
                        speed,
                        direction.display_name(),
                        color_mode.display_name(),
                        palette_index,
                        session_key.value(),
                    );
                } else {
                    println!(
                        "sent backlight {} effect color={} brightness={} speed={} direction={} mode={} palette={} key=0x{:02x} after retry {}",
                        effect.display_name(),
                        normalized_hex,
                        brightness,
                        speed,
                        direction.display_name(),
                        color_mode.display_name(),
                        palette_index,
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
    effect: RgbEffect,
    color: RgbColor,
    brightness: u8,
    speed: u8,
    direction: RgbDirection,
    color_mode: RgbColorMode,
    palette_index: u8,
) -> Result<SessionKey> {
    let dev = open_selected_device(api, selector)?;
    clear_input_reports(&dev)?;

    let protocol = KeyboardProtocol::new(&dev)?;
    protocol.set_main_light(
        effect,
        color,
        brightness,
        speed,
        direction,
        color_mode,
        palette_index,
    )?;

    Ok(protocol.session_key())
}

fn run_rgb_side(
    api: &HidApi,
    selector: &DeviceSelector,
    effect: RgbSideEffect,
    color_hex: Option<&str>,
    brightness: u8,
    speed: u8,
    color_mode: RgbColorMode,
    palette_index: u8,
) -> Result<()> {
    let (color, normalized_hex) = parse_effect_color(effect, color_hex)?;

    for attempt in 1..=RGB_SET_MAX_ATTEMPTS {
        match run_rgb_side_once(
            api,
            selector,
            effect,
            color,
            brightness,
            speed,
            color_mode,
            palette_index,
        ) {
            Ok(session_key) => {
                if attempt == 1 {
                    println!(
                        "sent side-light {} effect color={} brightness={} speed={} mode={} palette={} key=0x{:02x}",
                        effect.display_name(),
                        normalized_hex,
                        brightness,
                        speed,
                        color_mode.display_name(),
                        palette_index,
                        session_key.value(),
                    );
                } else {
                    println!(
                        "sent side-light {} effect color={} brightness={} speed={} mode={} palette={} key=0x{:02x} after retry {}",
                        effect.display_name(),
                        normalized_hex,
                        brightness,
                        speed,
                        color_mode.display_name(),
                        palette_index,
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

    bail!("unexpected side-light retry loop exit")
}

fn run_rgb_side_once(
    api: &HidApi,
    selector: &DeviceSelector,
    effect: RgbSideEffect,
    color: RgbColor,
    brightness: u8,
    speed: u8,
    color_mode: RgbColorMode,
    palette_index: u8,
) -> Result<SessionKey> {
    let dev = open_selected_device(api, selector)?;
    clear_input_reports(&dev)?;

    let protocol = KeyboardProtocol::new(&dev)?;
    protocol.set_side_light(effect, color, brightness, speed, color_mode, palette_index)?;

    Ok(protocol.session_key())
}

#[allow(clippy::too_many_arguments)]
fn run_rgb_decorative(
    api: &HidApi,
    selector: &DeviceSelector,
    effect: RgbSideEffect,
    color_hex: Option<&str>,
    brightness: u8,
    speed: u8,
    color_mode: RgbColorMode,
    palette_index: u8,
    base_offset: u16,
) -> Result<()> {
    let (color, normalized_hex) = parse_effect_color(effect, color_hex)?;

    for attempt in 1..=RGB_SET_MAX_ATTEMPTS {
        match run_rgb_decorative_once(
            api,
            selector,
            effect,
            color,
            brightness,
            speed,
            color_mode,
            palette_index,
            base_offset,
        ) {
            Ok(session_key) => {
                if attempt == 1 {
                    println!(
                        "sent decorative-light {} effect color={} brightness={} speed={} mode={} palette={} base_offset={} key=0x{:02x}",
                        effect.display_name(),
                        normalized_hex,
                        brightness,
                        speed,
                        color_mode.display_name(),
                        palette_index,
                        base_offset,
                        session_key.value(),
                    );
                } else {
                    println!(
                        "sent decorative-light {} effect color={} brightness={} speed={} mode={} palette={} base_offset={} key=0x{:02x} after retry {}",
                        effect.display_name(),
                        normalized_hex,
                        brightness,
                        speed,
                        color_mode.display_name(),
                        palette_index,
                        base_offset,
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

    bail!("unexpected decorative-light retry loop exit")
}

#[allow(clippy::too_many_arguments)]
fn run_rgb_decorative_once(
    api: &HidApi,
    selector: &DeviceSelector,
    effect: RgbSideEffect,
    color: RgbColor,
    brightness: u8,
    speed: u8,
    color_mode: RgbColorMode,
    palette_index: u8,
    base_offset: u16,
) -> Result<SessionKey> {
    let dev = open_selected_device(api, selector)?;
    clear_input_reports(&dev)?;

    let protocol = KeyboardProtocol::new(&dev)?;
    protocol.set_decorative_light(
        effect,
        color,
        brightness,
        speed,
        color_mode,
        palette_index,
        base_offset,
    )?;

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

fn parse_effect_color(
    effect: RgbSideEffect,
    color_hex: Option<&str>,
) -> Result<(RgbColor, String)> {
    if let Some(hex) = color_hex {
        return RgbColor::from_hex(hex);
    }

    if effect.supports_custom_color() {
        bail!(
            "--hex is required for {} effect (this effect uses custom RGB color)",
            effect.display_name()
        );
    }

    Ok((RgbColor { r: 0, g: 0, b: 0 }, String::from("auto")))
}

fn print_commands() {
    const COMMANDS: &[&str] = &["list", "rgb set", "rgb side", "rgb decorative", "raw send"];

    for command in COMMANDS {
        println!("{command}");
    }
}
