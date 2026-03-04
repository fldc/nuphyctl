mod cli;
mod color;
mod hid_io;
mod protocol;

use anyhow::{bail, Context, Result};
use clap::Parser;
use hidapi::HidApi;

use crate::cli::{Cli, Command, RawSubcommand, RgbSubcommand};
use crate::color::Rgb;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let api = HidApi::new().context("failed to initialize hidapi")?;

    match cli.command {
        Command::List => hid_io::list_devices(&api),
        Command::Rgb(rgb) => match rgb.action {
            RgbSubcommand::Set(args) => run_rgb_set(&api, args),
        },
        Command::Raw(raw) => match raw.action {
            RawSubcommand::Send(args) => run_raw_send(&api, args),
        },
    }
}

fn run_rgb_set(api: &HidApi, args: cli::RgbSetArgs) -> Result<()> {
    let color = Rgb::parse(&args.hex)?;
    let encoded = color.encoded();

    let set_packet = protocol::build_rgb_set_packet(encoded);
    let apply_packet = protocol::build_rgb_apply_packet();

    let dev = hid_io::open_selected_device(api, &args.device)
        .context("failed to open selected HID device for rgb set")?;
    hid_io::send_report(&dev, 0, &set_packet).context("failed to send RGB set packet")?;
    hid_io::send_report(&dev, 0, &apply_packet).context("failed to send RGB apply packet")?;

    println!(
        "sent static color {} (rgb={}, {}, {} encoded={:#04x}, {:#04x}, {:#04x})",
        color.hex_lower(),
        color.r,
        color.g,
        color.b,
        encoded.r,
        encoded.g,
        encoded.b
    );

    Ok(())
}

fn run_raw_send(api: &HidApi, args: cli::RawSendArgs) -> Result<()> {
    let bytes = parse_hex_bytes(&args.hex)?;
    if bytes.len() != protocol::REPORT_LEN {
        bail!(
            "raw report must be exactly {} bytes, got {}",
            protocol::REPORT_LEN,
            bytes.len()
        );
    }

    let dev = hid_io::open_selected_device(api, &args.device)
        .context("failed to open selected HID device for raw send")?;
    hid_io::send_report(&dev, args.report_id, &bytes).context("failed to send raw report")?;

    println!(
        "sent raw report_id={} bytes={}",
        args.report_id,
        bytes.len()
    );

    Ok(())
}

fn parse_hex_bytes(input: &str) -> Result<Vec<u8>> {
    let compact: String = input.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    if compact.is_empty() {
        bail!("empty hex string");
    }
    if compact.len() % 2 != 0 {
        bail!("hex string has odd length");
    }
    if !compact.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("hex string contains non-hex characters");
    }

    let mut out = Vec::with_capacity(compact.len() / 2);
    for i in (0..compact.len()).step_by(2) {
        out.push(
            u8::from_str_radix(&compact[i..i + 2], 16)
                .with_context(|| format!("invalid hex byte at offset {}", i))?,
        );
    }

    Ok(out)
}
