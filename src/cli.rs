use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "nuphyctl", about = "NuPhy keyboard control CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// List HID devices visible to hidapi
    List,
    /// RGB-related commands
    Rgb(RgbCommand),
    /// Send a raw HID output report
    Raw(RawCommand),
}

#[derive(Args, Debug)]
pub struct DeviceSelector {
    /// USB vendor ID (hex like 0x19f5 or decimal)
    #[arg(long, value_parser = parse_u16_any_base)]
    pub vid: Option<u16>,
    /// USB product ID (hex like 0x3245 or decimal)
    #[arg(long, value_parser = parse_u16_any_base)]
    pub pid: Option<u16>,
    /// hidraw path (for example /dev/hidraw5)
    #[arg(long)]
    pub path: Option<String>,
    /// HID interface number
    #[arg(long)]
    pub iface: Option<i32>,
    /// HID usage page (hex like 0x0001 or decimal)
    #[arg(long, value_parser = parse_u16_any_base)]
    pub usage_page: Option<u16>,
    /// HID usage (hex like 0x0000 or decimal)
    #[arg(long, value_parser = parse_u16_any_base)]
    pub usage: Option<u16>,
}

#[derive(Subcommand, Debug)]
pub enum RgbSubcommand {
    /// Set static color using #RRGGBB or RRGGBB
    Set(RgbSetArgs),
}

#[derive(Args, Debug)]
pub struct RgbCommand {
    #[command(subcommand)]
    pub action: RgbSubcommand,
}

#[derive(Args, Debug)]
pub struct RgbSetArgs {
    /// Color in #RRGGBB or RRGGBB format
    #[arg(long)]
    pub hex: String,

    /// Brightness in percent (0-100)
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u8).range(0..=100))]
    pub brightness: u8,

    #[command(flatten)]
    pub device: DeviceSelector,
}

#[derive(Subcommand, Debug)]
pub enum RawSubcommand {
    /// Send a raw output report (64 bytes)
    Send(RawSendArgs),
}

#[derive(Args, Debug)]
pub struct RawCommand {
    #[command(subcommand)]
    pub action: RawSubcommand,
}

#[derive(Args, Debug)]
pub struct RawSendArgs {
    /// Report payload bytes (space-separated hex or 128 hex chars)
    #[arg(long)]
    pub hex: String,

    /// HID report ID (NuPhy packets use 0)
    #[arg(long, default_value_t = 0)]
    pub report_id: u8,

    #[command(flatten)]
    pub device: DeviceSelector,
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
