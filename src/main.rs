use anyhow::{anyhow, bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use hidapi::{DeviceInfo, HidApi, HidDevice};
use std::{thread, time::Duration};

const REPORT_LEN: usize = 64;

#[derive(Parser, Debug)]
#[command(name = "nuphyctl", about = "NuPhy keyboard control CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// List HID devices visible to hidapi
    List,
    /// RGB-related commands
    Rgb(RgbCommand),
    /// Send a raw HID output report
    Raw(RawCommand),
}

#[derive(Args, Debug)]
struct DeviceSelector {
    /// USB vendor ID (hex like 0x19f5 or decimal)
    #[arg(long, value_parser = parse_u16_any_base)]
    vid: Option<u16>,
    /// USB product ID (hex like 0x3245 or decimal)
    #[arg(long, value_parser = parse_u16_any_base)]
    pid: Option<u16>,
    /// hidraw path (for example /dev/hidraw5)
    #[arg(long)]
    path: Option<String>,
    /// HID interface number
    #[arg(long)]
    iface: Option<i32>,
    /// HID usage page (hex like 0x0001 or decimal)
    #[arg(long, value_parser = parse_u16_any_base)]
    usage_page: Option<u16>,
    /// HID usage (hex like 0x0000 or decimal)
    #[arg(long, value_parser = parse_u16_any_base)]
    usage: Option<u16>,
}

#[derive(Subcommand, Debug)]
enum RgbSubcommand {
    /// Set static color using #RRGGBB or RRGGBB
    Set(RgbSetArgs),
}

#[derive(Args, Debug)]
struct RgbCommand {
    #[command(subcommand)]
    action: RgbSubcommand,
}

#[derive(Args, Debug)]
struct RgbSetArgs {
    /// Color in #RRGGBB or RRGGBB format
    #[arg(long)]
    hex: String,

    #[command(flatten)]
    device: DeviceSelector,
}

#[derive(Subcommand, Debug)]
enum RawSubcommand {
    /// Send a raw output report (64 bytes)
    Send(RawSendArgs),
}

#[derive(Args, Debug)]
struct RawCommand {
    #[command(subcommand)]
    action: RawSubcommand,
}

#[derive(Args, Debug)]
struct RawSendArgs {
    /// Report payload bytes (space-separated hex or 128 hex chars)
    #[arg(long)]
    hex: String,

    /// HID report ID (NuPhy packets use 0)
    #[arg(long, default_value_t = 0)]
    report_id: u8,

    #[command(flatten)]
    device: DeviceSelector,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let api = HidApi::new().context("failed to initialize hidapi")?;

    match cli.command {
        Command::List => list_devices(&api),
        Command::Rgb(rgb) => match rgb.action {
            RgbSubcommand::Set(args) => run_rgb_set(&api, args),
        },
        Command::Raw(raw) => match raw.action {
            RawSubcommand::Send(args) => run_raw_send(&api, args),
        },
    }
}

fn list_devices(api: &HidApi) -> Result<()> {
    for d in api.device_list() {
        println!(
            "path={:?} vid=0x{:04x} pid=0x{:04x} usage_page=0x{:04x} usage=0x{:04x} iface={} product={:?} manufacturer={:?} serial={:?}",
            d.path(),
            d.vendor_id(),
            d.product_id(),
            d.usage_page(),
            d.usage(),
            d.interface_number(),
            d.product_string(),
            d.manufacturer_string(),
            d.serial_number(),
        );
    }
    Ok(())
}

fn run_rgb_set(api: &HidApi, args: RgbSetArgs) -> Result<()> {
    let (r, g, b) = parse_rgb_hex(&args.hex)?;
    let er = encode_channel(r);
    let eg = encode_channel(g);
    let eb = encode_channel(b);

    let mut set_packet = [0u8; REPORT_LEN];
    set_packet[0] = 0x55;
    set_packet[1] = 0xd6;
    set_packet[2] = 0x00;
    set_packet[4] = 0x6d;
    set_packet[5] = 0x64;
    set_packet[6] = 0x64;
    set_packet[7] = 0x65;
    set_packet[8] = 0x67;
    set_packet[9] = 0x00;
    set_packet[10] = 0x66;
    set_packet[11] = 0x64;
    set_packet[12] = 0x64;
    set_packet[13] = 0x64;
    set_packet[14] = er;
    set_packet[15] = eg;
    set_packet[16] = eb;
    set_packet[3] = er.wrapping_add(eg).wrapping_add(eb).wrapping_add(0x93);

    let mut apply_packet = [0u8; REPORT_LEN];
    apply_packet[0] = 0x55;
    apply_packet[1] = 0xd6;
    apply_packet[2] = 0x00;
    apply_packet[3] = 0x93;
    apply_packet[4] = 0x65;
    apply_packet[5] = 0x65;
    apply_packet[6] = 0x64;
    apply_packet[7] = 0x65;

    let dev = open_selected_device(api, &args.device)
        .context("failed to open selected HID device for rgb set")?;
    send_report(&dev, 0, &set_packet).context("failed to send RGB set packet")?;
    thread::sleep(Duration::from_millis(8));
    send_report(&dev, 0, &apply_packet).context("failed to send RGB apply packet")?;

    println!(
        "sent static color {} (rgb={}, {}, {} encoded={:#04x}, {:#04x}, {:#04x})",
        normalize_hex(&args.hex)?,
        r,
        g,
        b,
        er,
        eg,
        eb
    );

    Ok(())
}

fn run_raw_send(api: &HidApi, args: RawSendArgs) -> Result<()> {
    let bytes = parse_hex_bytes(&args.hex)?;
    if bytes.len() != REPORT_LEN {
        bail!(
            "raw report must be exactly {} bytes, got {}",
            REPORT_LEN,
            bytes.len()
        );
    }

    let dev = open_selected_device(api, &args.device)
        .context("failed to open selected HID device for raw send")?;
    send_report(&dev, args.report_id, &bytes).context("failed to send raw report")?;

    println!(
        "sent raw report_id={} bytes={}",
        args.report_id,
        bytes.len()
    );
    Ok(())
}

fn open_selected_device(api: &HidApi, selector: &DeviceSelector) -> Result<HidDevice> {
    let selected: Vec<&DeviceInfo> = api
        .device_list()
        .filter(|d| {
            let vid_pid_match = match (selector.vid, selector.pid) {
                (Some(vid), Some(pid)) => d.vendor_id() == vid && d.product_id() == pid,
                (Some(vid), None) => d.vendor_id() == vid,
                (None, Some(pid)) => d.product_id() == pid,
                (None, None) => true,
            };
            let path_match = selector
                .path
                .as_ref()
                .map(|p| d.path().to_string_lossy() == p.as_str())
                .unwrap_or(true);
            let iface_match = selector
                .iface
                .map(|iface| d.interface_number() == iface)
                .unwrap_or(true);
            let usage_page_match = selector
                .usage_page
                .map(|usage_page| d.usage_page() == usage_page)
                .unwrap_or(true);
            let usage_match = selector
                .usage
                .map(|usage| d.usage() == usage)
                .unwrap_or(true);
            vid_pid_match && path_match && iface_match && usage_page_match && usage_match
        })
        .collect();

    if selected.is_empty() {
        bail!("no matching HID device found; try `nuphyctl list`");
    }

    if selector.path.is_none()
        && selector.iface.is_none()
        && selector.usage_page.is_none()
        && selector.usage.is_none()
    {
        let mut ranked: Vec<(&DeviceInfo, i32)> = selected
            .iter()
            .copied()
            .map(|d| {
                let mut score = 0;
                if d.usage_page() == 0x0001 && d.usage() == 0x0000 {
                    score += 100;
                }
                if d.interface_number() == 3 {
                    score += 50;
                }
                if d.usage_page() == 0x0001 && d.usage() == 0x0080 {
                    score -= 20;
                }
                if d.usage_page() == 0x0001 && d.usage() == 0x0006 {
                    score -= 30;
                }
                (d, score)
            })
            .collect();

        ranked.sort_by(|a, b| b.1.cmp(&a.1));

        if let Some((best, best_score)) = ranked.first().copied() {
            let next_score = ranked.get(1).map(|(_, s)| *s).unwrap_or(i32::MIN);
            if best_score > 0 && best_score > next_score {
                return api
                    .open_path(best.path())
                    .with_context(|| format!("open failed for path {:?}", best.path()));
            }
        }
    }

    let unique_paths: std::collections::BTreeSet<String> = selected
        .iter()
        .map(|d| d.path().to_string_lossy().into_owned())
        .collect();

    if unique_paths.len() == 1 {
        let only = selected[0];
        return api
            .open_path(only.path())
            .with_context(|| format!("open failed for path {:?}", only.path()));
    }

    match selected.len() {
        1 => {
            let d = selected[0];
            api.open_path(d.path())
                .with_context(|| format!("open failed for path {:?}", d.path()))
        }
        _ => {
            let mut lines = String::new();
            for d in selected {
                lines.push_str(&format!(
                    "path={:?} vid=0x{:04x} pid=0x{:04x} usage_page=0x{:04x} usage=0x{:04x} iface={} product={:?}\n",
                    d.path(),
                    d.vendor_id(),
                    d.product_id(),
                    d.usage_page(),
                    d.usage(),
                    d.interface_number(),
                    d.product_string(),
                ));
            }
            bail!(
                "multiple matching HID devices; specify --path or narrow with --iface/--usage-page/--usage. candidates:\n{}",
                lines
            )
        }
    }
}

fn send_report(dev: &HidDevice, report_id: u8, data: &[u8]) -> Result<()> {
    let mut with_rid = Vec::with_capacity(data.len() + 1);
    with_rid.push(report_id);
    with_rid.extend_from_slice(data);

    let mut errors = Vec::new();

    match dev.send_output_report(&with_rid) {
        Ok(()) => return Ok(()),
        Err(err) => errors.push(format!("send_output_report(with_rid) failed: {err}")),
    }

    match dev.write(&with_rid) {
        Ok(written) if written == with_rid.len() => return Ok(()),
        Ok(written) => errors.push(format!(
            "write(with_rid) short write: wrote {} bytes, expected {}",
            written,
            with_rid.len()
        )),
        Err(err) => errors.push(format!("write(with_rid) failed: {err}")),
    }

    // Some hidraw paths expect raw payload without report-id prefix for report 0.
    if report_id == 0 {
        match dev.write(data) {
            Ok(written) if written == data.len() => return Ok(()),
            Ok(written) => errors.push(format!(
                "write(no_rid) short write: wrote {} bytes, expected {}",
                written,
                data.len()
            )),
            Err(err) => errors.push(format!("write(no_rid) failed: {err}")),
        }
    }

    Err(anyhow!(errors.join("; ")))
}

fn parse_rgb_hex(input: &str) -> Result<(u8, u8, u8)> {
    let normalized = normalize_hex(input)?;
    let value = u32::from_str_radix(&normalized, 16)
        .with_context(|| format!("failed to parse color hex: {}", input))?;
    let r = ((value >> 16) & 0xff) as u8;
    let g = ((value >> 8) & 0xff) as u8;
    let b = (value & 0xff) as u8;
    Ok((r, g, b))
}

fn normalize_hex(input: &str) -> Result<String> {
    let v = input.strip_prefix('#').unwrap_or(input);
    if v.len() != 6 || !v.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("color must be in #RRGGBB or RRGGBB format");
    }
    Ok(v.to_ascii_lowercase())
}

fn encode_channel(v: u8) -> u8 {
    let min = 0x64u16;
    let range = 0x9b_u16 - min;
    let mapped = min + ((u16::from(v) * range + 127) / 255);
    mapped as u8
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
        out.push(u8::from_str_radix(&compact[i..i + 2], 16)?);
    }
    Ok(out)
}

fn parse_u16_any_base(input: &str) -> Result<u16, String> {
    if let Some(hex) = input
        .strip_prefix("0x")
        .or_else(|| input.strip_prefix("0X"))
    {
        u16::from_str_radix(hex, 16).map_err(|e| e.to_string())
    } else {
        input.parse::<u16>().map_err(|e| e.to_string())
    }
}
