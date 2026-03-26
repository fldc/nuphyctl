use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "nuphyctl", about = "NuPhy keyboard control CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Print all available command paths
    Commands,
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
    /// Set backlight effect and color using #RRGGBB or RRGGBB
    Set(RgbSetArgs),
    /// Set side-light effect and color
    Side(RgbSideSetArgs),
    /// Set decorative-light effect and color (experimental)
    Decorative(RgbDecorativeSetArgs),
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum RgbEffect {
    Ray,
    Stair,
    Static,
    Breath,
    Flower,
    Wave,
    Ripple,
    Spout,
    Galaxy,
    Rotation,
    #[value(name = "ripple2", alias = "ripple-2", alias = "ripple-alt")]
    Ripple2,
    Point,
    Grid,
    Time,
    Rain,
    Ribbon,
    Gaming,
    Identify,
    Windmill,
    Diagonal,
}

impl RgbEffect {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Ray => "Ray",
            Self::Stair => "Stair",
            Self::Static => "Static",
            Self::Breath => "Breath",
            Self::Flower => "Flower",
            Self::Wave => "Wave",
            Self::Ripple => "Ripple",
            Self::Spout => "Spout",
            Self::Galaxy => "Galaxy",
            Self::Rotation => "Rotation",
            Self::Ripple2 => "Ripple (2nd)",
            Self::Point => "Point",
            Self::Grid => "Grid",
            Self::Time => "Time",
            Self::Rain => "Rain",
            Self::Ribbon => "Ribbon",
            Self::Gaming => "Gaming",
            Self::Identify => "Identify",
            Self::Windmill => "Windmill",
            Self::Diagonal => "Diagonal",
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum RgbSideEffect {
    Time,
    Neon,
    Static,
    #[value(alias = "breath")]
    Breathe,
    Rhythm,
}

impl RgbSideEffect {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Time => "Time",
            Self::Neon => "Neon",
            Self::Static => "Static",
            Self::Breathe => "Breathe",
            Self::Rhythm => "Rhythm",
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum RgbDirection {
    Left,
    Right,
}

impl RgbDirection {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum RgbColorMode {
    Custom,
    Preset,
}

impl RgbColorMode {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Custom => "custom",
            Self::Preset => "preset",
        }
    }
}

#[derive(Args, Debug)]
pub struct RgbCommand {
    #[command(subcommand)]
    pub action: RgbSubcommand,
}

#[derive(Args, Debug)]
pub struct RgbSetArgs {
    /// Lighting effect (defaults to static)
    #[arg(long, value_enum, default_value_t = RgbEffect::Static)]
    pub effect: RgbEffect,

    /// Color in #RRGGBB or RRGGBB format
    #[arg(long)]
    pub hex: String,

    /// Brightness in percent (0-100)
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u8).range(0..=100))]
    pub brightness: u8,

    /// Animation speed gear (0-4)
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(0..=4))]
    pub speed: u8,

    /// Effect direction
    #[arg(long, value_enum, default_value_t = RgbDirection::Right)]
    pub direction: RgbDirection,

    /// Color source (custom RGB or preset/palette)
    #[arg(long, value_enum, default_value_t = RgbColorMode::Custom)]
    pub color_mode: RgbColorMode,

    /// Palette index used when --color-mode preset
    #[arg(long, default_value_t = 0)]
    pub palette_index: u8,

    #[command(flatten)]
    pub device: DeviceSelector,
}

#[derive(Args, Debug)]
pub struct RgbSideSetArgs {
    /// Side-light effect
    #[arg(long, value_enum, default_value_t = RgbSideEffect::Static)]
    pub effect: RgbSideEffect,

    /// Color in #RRGGBB or RRGGBB format
    #[arg(long)]
    pub hex: String,

    /// Brightness in percent (0-100)
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u8).range(0..=100))]
    pub brightness: u8,

    /// Animation speed gear (0-4)
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(0..=4))]
    pub speed: u8,

    /// Color source (custom RGB or preset/palette)
    #[arg(long, value_enum, default_value_t = RgbColorMode::Custom)]
    pub color_mode: RgbColorMode,

    /// Palette index used when --color-mode preset
    #[arg(long, default_value_t = 0)]
    pub palette_index: u8,

    #[command(flatten)]
    pub device: DeviceSelector,
}

#[derive(Args, Debug)]
pub struct RgbDecorativeSetArgs {
    /// Decorative-light effect
    #[arg(long, value_enum, default_value_t = RgbSideEffect::Static)]
    pub effect: RgbSideEffect,

    /// Color in #RRGGBB or RRGGBB format
    #[arg(long)]
    pub hex: String,

    /// Brightness in percent (0-100)
    #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u8).range(0..=100))]
    pub brightness: u8,

    /// Animation speed gear (0-4)
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(0..=4))]
    pub speed: u8,

    /// Color source (custom RGB or preset/palette)
    #[arg(long, value_enum, default_value_t = RgbColorMode::Custom)]
    pub color_mode: RgbColorMode,

    /// Palette index used when --color-mode preset
    #[arg(long, default_value_t = 0)]
    pub palette_index: u8,

    /// Decorative-light base offset (model-dependent)
    #[arg(long, value_parser = clap::value_parser!(u16).range(0..=255), default_value_t = 17)]
    pub base_offset: u16,

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
