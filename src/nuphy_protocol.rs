use crate::cli::{RgbColorMode, RgbDirection, RgbEffect, RgbSideEffect};
use crate::color::RgbColor;
use crate::hid_transport::{HidResponder, REPORT_LEN, Report};
use anyhow::{Context, Result, bail};
use hidapi::HidDevice;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const KEY_EXCHANGE_PAYLOAD_LEN: usize = REPORT_LEN - 8;
const DEFAULT_TIMEOUT: Duration = Duration::from_millis(1000);

const CMD_SET_DATA: u8 = 0xd6;
const CMD_APPLY: u8 = 0xd5;
const CMD_KEY_EXCHANGE: u8 = 0xee;

const CMD_GET_BASE: u8 = 0xa0;
const CMD_GET_FIRMWARE_INFO: u8 = 0xa1;
const CMD_GET_DEFAULT_KEYS: u8 = 0xb1;
const CMD_GET_USE_KEYS: u8 = 0xb2;
const CMD_SET_USE_KEYS: u8 = 0xb3;
const CMD_RESTORE_USE_LAYERS: u8 = 0xb4;

const DATA_INFO_LEN: u8 = 8;
const DATA_COMMAND_ATTEMPTS: usize = 3;

const SUBCMD_MAIN_LIGHT: u8 = 0x09;
const SUBCMD_SIDE_LIGHT: u8 = 0x08;
const SUBCMD_MAIN_BRIGHTNESS: u8 = 0x01;
const SUBCMD_APPLY: u8 = 0x11;

const MAIN_LIGHT_OFFSET: u16 = 0;
const SIDE_LIGHT_OFFSET: u16 = 9;
const MAIN_BRIGHTNESS_OFFSET: u16 = 1;
const SET_DATA_WINDOW_SIZE: u8 = 1;

type KeyExchangePayload = [u8; KEY_EXCHANGE_PAYLOAD_LEN];

#[derive(Clone, Copy, Debug)]
pub struct MainLightSettings {
    pub effect: RgbEffect,
    pub color: RgbColor,
    pub brightness: u8,
    pub speed: u8,
    pub direction: RgbDirection,
    pub color_mode: RgbColorMode,
    pub palette_index: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct SideLightSettings {
    pub effect: RgbSideEffect,
    pub color: RgbColor,
    pub brightness: u8,
    pub speed: u8,
    pub color_mode: RgbColorMode,
    pub palette_index: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct SessionKey(u8);

impl SessionKey {
    pub const fn value(self) -> u8 {
        self.0
    }
}

pub struct KeyboardProtocol<'a> {
    dev: &'a HidDevice,
    session_key: SessionKey,
    responder: HidResponder,
}

impl<'a> KeyboardProtocol<'a> {
    pub fn new(dev: &'a HidDevice) -> Result<Self> {
        let responder = HidResponder::with_timeout(DEFAULT_TIMEOUT);
        let session_key = negotiate_session_key(dev, &responder)
            .context("failed to negotiate HID session key")?;
        Ok(Self {
            dev,
            session_key,
            responder,
        })
    }

    pub fn session_key(&self) -> SessionKey {
        self.session_key
    }

    pub fn set_main_light(&self, settings: MainLightSettings) -> Result<()> {
        let effect_id = settings.effect.protocol_id();
        let light_payload = build_main_light_payload(
            effect_id,
            settings.brightness,
            settings.speed,
            settings.direction.protocol_value(),
            settings.color_mode,
            settings.palette_index,
            settings.color,
        );
        self.send_set_data(
            SUBCMD_MAIN_LIGHT,
            MAIN_LIGHT_OFFSET,
            &light_payload,
            SET_DATA_WINDOW_SIZE,
        )
        .context("failed to send RGB main-light packet")?;

        let brightness_payload = [settings.brightness];
        self.send_set_data(
            SUBCMD_MAIN_BRIGHTNESS,
            MAIN_BRIGHTNESS_OFFSET,
            &brightness_payload,
            SET_DATA_WINDOW_SIZE,
        )
        .context("failed to send RGB brightness packet")?;

        self.apply().context("failed to send RGB apply packet")
    }

    pub fn set_side_light(&self, settings: SideLightSettings) -> Result<()> {
        let light_payload = build_side_light_payload(
            settings.effect.protocol_id(),
            settings.brightness,
            settings.speed,
            settings.color_mode,
            settings.palette_index,
            settings.color,
        );
        self.send_set_data(
            SUBCMD_SIDE_LIGHT,
            SIDE_LIGHT_OFFSET,
            &light_payload,
            SET_DATA_WINDOW_SIZE,
        )
        .context("failed to send side-light packet")?;

        let brightness_payload = [settings.brightness];
        self.send_set_data(
            SUBCMD_MAIN_BRIGHTNESS,
            SIDE_LIGHT_OFFSET + 1,
            &brightness_payload,
            SET_DATA_WINDOW_SIZE,
        )
        .context("failed to send side-light brightness packet")?;

        Ok(())
    }

    pub fn set_decorative_light(
        &self,
        settings: SideLightSettings,
        base_offset: u16,
    ) -> Result<()> {
        let light_payload = build_side_light_payload(
            settings.effect.protocol_id(),
            settings.brightness,
            settings.speed,
            settings.color_mode,
            settings.palette_index,
            settings.color,
        );
        self.send_set_data(
            SUBCMD_SIDE_LIGHT,
            base_offset,
            &light_payload,
            SET_DATA_WINDOW_SIZE,
        )
        .context("failed to send decorative-light packet")?;

        let brightness_payload = [settings.brightness];
        self.send_set_data(
            SUBCMD_MAIN_BRIGHTNESS,
            base_offset + 1,
            &brightness_payload,
            SET_DATA_WINDOW_SIZE,
        )
        .context("failed to send decorative-light brightness packet")?;

        Ok(())
    }

    /// `GetBase` (`0xa0`): mode, layer count, layers per mode, rows, cols, extras.
    pub fn get_base(&self) -> Result<Vec<u8>> {
        self.data_command(CMD_GET_BASE, 0, DATA_INFO_LEN, &[])
            .context("GetBase (0xa0) failed")
    }

    /// `GetFirmwareInfo` (`0xa1`).
    pub fn get_firmware_info(&self) -> Result<Vec<u8>> {
        self.data_command(CMD_GET_FIRMWARE_INFO, 0, DATA_INFO_LEN, &[])
            .context("GetFirmwareInfo (0xa1) failed")
    }

    /// Read `len` bytes of the active (`GetUseKeys`, `0xb2`) or factory
    /// (`GetDefaultKeys`, `0xb1`) keymap, starting at byte 0.
    pub fn read_keymap(&self, factory: bool, len: usize) -> Result<Vec<u8>> {
        let cmd = if factory {
            CMD_GET_DEFAULT_KEYS
        } else {
            CMD_GET_USE_KEYS
        };
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            let chunk = (len - out.len()).min(KEY_EXCHANGE_PAYLOAD_LEN);
            let addr = u16::try_from(out.len()).context("keymap address exceeds 16 bits")?;
            let data = self
                .data_command(cmd, addr, chunk as u8, &[])
                .with_context(|| format!("keymap read (0x{cmd:02x}) at byte {addr} failed"))?;
            out.extend_from_slice(&data);
        }
        Ok(out)
    }

    /// `SetUseKeys` (`0xb3`): write raw keymap bytes at a byte address.
    pub fn write_keymap(&self, addr: u16, data: &[u8]) -> Result<()> {
        if data.is_empty() || data.len() > KEY_EXCHANGE_PAYLOAD_LEN {
            bail!(
                "keymap write must be 1..={} bytes",
                KEY_EXCHANGE_PAYLOAD_LEN
            );
        }
        self.data_command(CMD_SET_USE_KEYS, addr, data.len() as u8, data)
            .with_context(|| format!("keymap write (0xb3) at byte {addr} failed"))?;
        Ok(())
    }

    /// `RestoreUseLayers` (`0xb4`): reset `count` layers starting at `first`
    /// to the factory keymap.
    pub fn restore_layers(&self, first: u8, count: u8) -> Result<()> {
        let zeros = vec![0u8; usize::from(count)];
        self.data_command(CMD_RESTORE_USE_LAYERS, u16::from(first), count, &zeros)
            .context("RestoreUseLayers (0xb4) failed")?;
        Ok(())
    }

    /// Send one data command and return its decoded payload. Retries on
    /// timeouts and malformed replies; every data command is idempotent.
    fn data_command(&self, cmd: u8, addr: u16, len: u8, data: &[u8]) -> Result<Vec<u8>> {
        let packet = build_data_packet(cmd, addr, len, data, self.session_key)?;
        let key = self.session_key;
        let mut last_err = None;
        for _ in 0..DATA_COMMAND_ATTEMPTS {
            let reply = self
                .responder
                .send_and_expect_match(self.dev, 0, &packet, |r| {
                    r[0] == 0xaa && r[1] == cmd && decode_data_header(r, key) == (len, addr)
                });
            match reply.and_then(|r| parse_data_response(&r, cmd, addr, len, key)) {
                Ok(payload) => return Ok(payload),
                Err(err) => last_err = Some(err),
            }
        }
        Err(last_err.expect("at least one attempt"))
    }

    fn send_set_data(&self, subcommand: u8, offset: u16, payload: &[u8], size: u8) -> Result<()> {
        let packet = build_protocol_packet(
            CMD_SET_DATA,
            subcommand,
            offset,
            payload,
            size,
            self.session_key,
        )?;
        let ack = self
            .responder
            .send_and_expect_ack(self.dev, 0, &packet, CMD_SET_DATA)?;
        validate_ack(&ack, CMD_SET_DATA)?;
        Ok(())
    }

    fn apply(&self) -> Result<()> {
        let packet = build_protocol_packet(CMD_APPLY, SUBCMD_APPLY, 0, &[], 0, self.session_key)?;
        let ack = self
            .responder
            .send_and_expect_ack(self.dev, 0, &packet, CMD_APPLY)?;
        validate_ack(&ack, CMD_APPLY)?;
        Ok(())
    }
}

impl RgbEffect {
    const fn protocol_id(self) -> u8 {
        match self {
            Self::Ray => 1,
            Self::Stair => 2,
            Self::Static => 3,
            Self::Breath => 4,
            Self::Flower => 5,
            Self::Wave => 6,
            Self::Ripple => 7,
            Self::Spout => 8,
            Self::Galaxy => 9,
            Self::Rotation => 10,
            Self::Ripple2 => 11,
            Self::Point => 12,
            Self::Grid => 13,
            Self::Time => 14,
            Self::Rain => 15,
            Self::Ribbon => 16,
            Self::Gaming => 17,
            Self::Identify => 18,
            Self::Windmill => 19,
            Self::Diagonal => 20,
        }
    }
}

impl RgbSideEffect {
    const fn protocol_id(self) -> u8 {
        match self {
            Self::Time => 0,
            Self::Neon => 1,
            Self::Static => 2,
            Self::Breathe => 3,
            Self::Rhythm => 4,
        }
    }
}

impl RgbDirection {
    const fn protocol_value(self) -> u8 {
        match self {
            Self::Left => 1,
            Self::Right => 0,
        }
    }
}

fn build_main_light_payload(
    effect_id: u8,
    brightness: u8,
    speed: u8,
    direction: u8,
    color_mode: RgbColorMode,
    palette_index: u8,
    color: RgbColor,
) -> [u8; 9] {
    let (mode_flag, palette) = match color_mode {
        RgbColorMode::Custom => (0, 0),
        RgbColorMode::Preset => (1, palette_index),
    };
    [
        effect_id, brightness, speed, direction, mode_flag, palette, color.r, color.g, color.b,
    ]
}

fn build_side_light_payload(
    effect_id: u8,
    brightness: u8,
    speed: u8,
    color_mode: RgbColorMode,
    palette_index: u8,
    color: RgbColor,
) -> [u8; 8] {
    let (mode_flag, palette) = match color_mode {
        RgbColorMode::Custom => (0, 0),
        RgbColorMode::Preset => (1, palette_index),
    };
    [
        effect_id, brightness, speed, mode_flag, palette, color.r, color.g, color.b,
    ]
}

fn negotiate_session_key(dev: &HidDevice, responder: &HidResponder) -> Result<SessionKey> {
    let challenge = build_key_exchange_challenge();
    let request = build_key_exchange_packet(&challenge);

    let response = responder
        .send_and_expect_ack(dev, 0, &request, CMD_KEY_EXCHANGE)
        .context("timed out waiting for key exchange response")?;

    if response[0] != 0xaa || response[1] != CMD_KEY_EXCHANGE {
        bail!(
            "unexpected key exchange response header: {:02x} {:02x}",
            response[0],
            response[1]
        );
    }

    let expected_checksum = calc_checksum(&response);
    if response[3] != expected_checksum {
        bail!(
            "invalid key exchange response checksum: got {:02x}, expected {:02x}",
            response[3],
            expected_checksum
        );
    }

    let key = SessionKey(response[4]);
    if response[5] != key.value() || response[6] != key.value() || response[7] != key.value() {
        bail!(
            "invalid key exchange response: key bytes mismatch ({:02x} {:02x} {:02x} {:02x})",
            response[4],
            response[5],
            response[6],
            response[7]
        );
    }

    let payload = &response[8..8 + KEY_EXCHANGE_PAYLOAD_LEN];
    for (idx, (&sent, &recv)) in challenge.iter().zip(payload.iter()).enumerate() {
        if (sent ^ recv) != key.value() {
            bail!(
                "invalid key exchange response payload at byte {} (sent={:02x}, recv={:02x}, key={:02x})",
                idx,
                sent,
                recv,
                key.value()
            );
        }
    }

    Ok(key)
}

fn validate_ack(ack: &Report, expected_cmd: u8) -> Result<()> {
    if ack[0] != 0xaa {
        bail!("invalid HID ack header: {:02x}", ack[0]);
    }
    if ack[1] != expected_cmd {
        bail!(
            "unexpected HID ack command: expected {:02x}, got {:02x}",
            expected_cmd,
            ack[1]
        );
    }
    let expected_checksum = calc_checksum(ack);
    if ack[3] != expected_checksum {
        bail!(
            "invalid HID ack checksum: got {:02x}, expected {:02x}",
            ack[3],
            expected_checksum
        );
    }
    Ok(())
}

fn build_key_exchange_packet(challenge: &KeyExchangePayload) -> Report {
    let mut packet = [0u8; REPORT_LEN];
    packet[0] = 0x55;
    packet[1] = CMD_KEY_EXCHANGE;
    packet[2] = 0x00;
    packet[8..].copy_from_slice(challenge);
    packet[3] = calc_checksum(&packet);
    packet
}

fn build_key_exchange_challenge() -> KeyExchangePayload {
    let mut out = [0u8; KEY_EXCHANGE_PAYLOAD_LEN];
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9e37_79b9_7f4a_7c15);

    if seed == 0 {
        seed = 0x9e37_79b9_7f4a_7c15;
    }

    for byte in &mut out {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        *byte = (seed & 0xff) as u8;
    }

    out
}

fn build_protocol_packet(
    command: u8,
    subcommand: u8,
    offset: u16,
    payload: &[u8],
    size: u8,
    key: SessionKey,
) -> Result<Report> {
    if payload.len() > KEY_EXCHANGE_PAYLOAD_LEN {
        bail!(
            "protocol payload too large: {} bytes (max {})",
            payload.len(),
            KEY_EXCHANGE_PAYLOAD_LEN
        );
    }

    let mut packet = [0u8; REPORT_LEN];
    packet[0] = 0x55;
    packet[1] = command;
    packet[2] = 0x00;
    packet[4] = subcommand ^ key.value();
    packet[5] = (offset as u8) ^ key.value();
    packet[6] = ((offset >> 8) as u8) ^ key.value();
    packet[7] = size ^ key.value();

    for (idx, b) in payload.iter().enumerate() {
        packet[8 + idx] = *b ^ key.value();
    }

    packet[3] = calc_checksum(&packet);
    Ok(packet)
}

/// Data-command request: `55 cmd 00 sum len^K addr_lo^K addr_hi^K 00^K data^K`.
/// Same framing as `build_protocol_packet`, with the length in byte 4.
fn build_data_packet(cmd: u8, addr: u16, len: u8, data: &[u8], key: SessionKey) -> Result<Report> {
    build_protocol_packet(cmd, len, addr, data, 0, key)
}

fn decode_data_header(report: &Report, key: SessionKey) -> (u8, u16) {
    let k = key.value();
    let len = report[4] ^ k;
    let addr = u16::from(report[5] ^ k) | (u16::from(report[6] ^ k) << 8);
    (len, addr)
}

/// Validate a data-command reply (`aa cmd .. sum len addr .. data`, all
/// fields after byte 3 XORed with the session key) and return its payload.
fn parse_data_response(
    report: &Report,
    cmd: u8,
    addr: u16,
    len: u8,
    key: SessionKey,
) -> Result<Vec<u8>> {
    validate_ack(report, cmd)?;
    let (got_len, got_addr) = decode_data_header(report, key);
    if got_len != len || got_addr != addr {
        bail!(
            "reply to 0x{cmd:02x} echoes len={got_len} addr={got_addr}, expected len={len} addr={addr}"
        );
    }
    let len = usize::from(len);
    if len > KEY_EXCHANGE_PAYLOAD_LEN {
        bail!("reply length {len} exceeds packet payload");
    }
    Ok(report[8..8 + len].iter().map(|b| b ^ key.value()).collect())
}

/// Firmware version from `GetFirmwareInfo`, formatted like NuPhy IO
/// (`major.minor.patch[.build]`).
pub fn format_firmware_version(info: &[u8]) -> String {
    if info.len() < 6 {
        return format!("{info:02x?}");
    }
    let mut version = format!("{}.{}.{}", info[2], info[1], info[0]);
    if info[3] == 0xaa {
        version.push_str(&format!(
            ".{}",
            u16::from(info[4]) | (u16::from(info[5]) << 8)
        ));
    }
    version
}

fn calc_checksum(packet: &Report) -> u8 {
    packet[4..]
        .iter()
        .copied()
        .fold(0u8, |acc, b| acc.wrapping_add(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_packet_without_key() {
        let p = build_data_packet(CMD_GET_USE_KEYS, 784, 56, &[], SessionKey(0)).unwrap();
        assert_eq!(
            &p[..8],
            &[0x55, 0xb2, 0x00, 56 + 0x10 + 0x03, 56, 0x10, 0x03, 0x00]
        );
        assert!(p[8..].iter().all(|&b| b == 0));
    }

    #[test]
    fn data_packet_xor_and_checksum() {
        let key = SessionKey(0x5a);
        let p = build_data_packet(CMD_SET_USE_KEYS, 0x0312, 2, &[0x00, 0xbe], key).unwrap();
        assert_eq!(p[0], 0x55);
        assert_eq!(p[1], 0xb3);
        assert_eq!(p[2], 0x00);
        assert_eq!(
            &p[4..10],
            &[2 ^ 0x5a, 0x12 ^ 0x5a, 0x03 ^ 0x5a, 0x5a, 0x5a, 0xbe ^ 0x5a]
        );
        // Unused payload bytes stay zero (not XORed), as in NuPhy IO.
        assert!(p[10..].iter().all(|&b| b == 0));
        let sum = p[4..].iter().fold(0u8, |a, &b| a.wrapping_add(b));
        assert_eq!(p[3], sum);
    }

    #[test]
    fn restore_packet_layout() {
        let p = build_data_packet(CMD_RESTORE_USE_LAYERS, 4, 4, &[0; 4], SessionKey(0)).unwrap();
        assert_eq!(
            &p[..12],
            &[0x55, 0xb4, 0x00, 0x08, 0x04, 0x04, 0x00, 0x00, 0, 0, 0, 0]
        );
    }

    fn reply(cmd: u8, len: u8, addr: u16, data: &[u8], key: u8) -> Report {
        let mut r = [0u8; REPORT_LEN];
        r[0] = 0xaa;
        r[1] = cmd;
        r[4] = len ^ key;
        r[5] = (addr as u8) ^ key;
        r[6] = ((addr >> 8) as u8) ^ key;
        r[7] = key;
        for (i, b) in data.iter().enumerate() {
            r[8 + i] = b ^ key;
        }
        r[3] = calc_checksum(&r);
        r
    }

    #[test]
    fn parses_data_reply_with_key() {
        let key = SessionKey(0x3c);
        let r = reply(0xb2, 4, 784, &[0x00, 0x29, 0x00, 0xbe], 0x3c);
        assert_eq!(decode_data_header(&r, key), (4, 784));
        let data = parse_data_response(&r, 0xb2, 784, 4, key).unwrap();
        assert_eq!(data, vec![0x00, 0x29, 0x00, 0xbe]);
    }

    #[test]
    fn rejects_bad_replies() {
        let key = SessionKey(0);
        let good = reply(0xb2, 2, 0, &[0, 0x29], 0);
        assert!(parse_data_response(&good, 0xb2, 0, 2, key).is_ok());
        assert!(parse_data_response(&good, 0xb2, 2, 2, key).is_err());
        assert!(parse_data_response(&good, 0xb1, 0, 2, key).is_err());
        let mut bad_sum = good;
        bad_sum[3] ^= 1;
        assert!(parse_data_response(&bad_sum, 0xb2, 0, 2, key).is_err());
        let mut bad_header = good;
        bad_header[0] = 0x55;
        bad_header[3] = calc_checksum(&bad_header);
        assert!(parse_data_response(&bad_header, 0xb2, 0, 2, key).is_err());
    }

    #[test]
    fn firmware_version() {
        assert_eq!(
            format_firmware_version(&[4, 0, 1, 0xaa, 5, 0, 0, 0]),
            "1.0.4.5"
        );
        assert_eq!(
            format_firmware_version(&[17, 0, 1, 0xaa, 6, 0, 0, 0]),
            "1.0.17.6"
        );
        assert_eq!(format_firmware_version(&[3, 2, 1, 0, 0, 0]), "1.2.3");
    }
}
