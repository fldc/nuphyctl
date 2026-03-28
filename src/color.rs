use anyhow::{Context, Result, bail};

#[derive(Clone, Copy, Debug)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub fn from_hex(input: &str) -> Result<(Self, String)> {
        let normalized = normalize_hex(input)?;
        let value = u32::from_str_radix(&normalized, 16)
            .with_context(|| format!("failed to parse color hex: {}", input))?;
        Ok((
            Self {
                r: ((value >> 16) & 0xff) as u8,
                g: ((value >> 8) & 0xff) as u8,
                b: (value & 0xff) as u8,
            },
            normalized,
        ))
    }
}

pub fn normalize_hex(input: &str) -> Result<String> {
    let v = input.strip_prefix('#').unwrap_or(input);
    if v.len() != 6 || !v.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("color must be in #RRGGBB or RRGGBB format");
    }
    Ok(v.to_ascii_lowercase())
}

pub fn parse_hex_bytes(input: &str) -> Result<Vec<u8>> {
    let compact: String = input.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    if compact.is_empty() {
        bail!("empty hex string");
    }
    if !compact.len().is_multiple_of(2) {
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
