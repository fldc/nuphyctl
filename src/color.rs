use anyhow::{bail, Context, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncodedRgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn parse(input: &str) -> Result<Self> {
        let normalized = normalize_hex(input)?;
        let value = u32::from_str_radix(&normalized, 16)
            .with_context(|| format!("failed to parse color hex: {input}"))?;
        Ok(Self {
            r: ((value >> 16) & 0xff) as u8,
            g: ((value >> 8) & 0xff) as u8,
            b: (value & 0xff) as u8,
        })
    }

    pub fn hex_lower(&self) -> String {
        format!("{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn encoded(&self) -> EncodedRgb {
        EncodedRgb {
            r: encode_channel(self.r),
            g: encode_channel(self.g),
            b: encode_channel(self.b),
        }
    }
}

fn normalize_hex(input: &str) -> Result<String> {
    let value = input.strip_prefix('#').unwrap_or(input);
    if value.len() != 6 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("color must be in #RRGGBB or RRGGBB format");
    }
    Ok(value.to_ascii_lowercase())
}

fn encode_channel(value: u8) -> u8 {
    const ENCODED_MIN: u16 = 0x64;
    const ENCODED_MAX: u16 = 0x9b;
    const INPUT_MAX: u16 = 255;

    let range = ENCODED_MAX - ENCODED_MIN;
    let mapped = ENCODED_MIN + ((u16::from(value) * range + 127) / INPUT_MAX);
    mapped as u8
}
