use crate::cli::{RgbColorMode, RgbDirection, RgbEffect, RgbSideEffect};
use crate::color::RgbColor;
use crate::hid_transport::{HidResponder, Report, REPORT_LEN};
use anyhow::{bail, Context, Result};
use hidapi::HidDevice;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const KEY_EXCHANGE_PAYLOAD_LEN: usize = REPORT_LEN - 8;
const DEFAULT_TIMEOUT: Duration = Duration::from_millis(1000);

const CMD_SET_DATA: u8 = 0xd6;
const CMD_APPLY: u8 = 0xd5;
const CMD_KEY_EXCHANGE: u8 = 0xee;

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

    pub fn set_main_light(
        &self,
        effect: RgbEffect,
        color: RgbColor,
        brightness: u8,
        speed: u8,
        direction: RgbDirection,
        color_mode: RgbColorMode,
        palette_index: u8,
    ) -> Result<()> {
        let effect_id = effect.protocol_id();
        let light_payload = build_main_light_payload(
            effect_id,
            brightness,
            speed,
            direction.protocol_value(),
            color_mode,
            palette_index,
            color,
        );
        self.send_set_data(
            SUBCMD_MAIN_LIGHT,
            MAIN_LIGHT_OFFSET,
            &light_payload,
            SET_DATA_WINDOW_SIZE,
        )
        .context("failed to send RGB main-light packet")?;

        let brightness_payload = [brightness];
        self.send_set_data(
            SUBCMD_MAIN_BRIGHTNESS,
            MAIN_BRIGHTNESS_OFFSET,
            &brightness_payload,
            SET_DATA_WINDOW_SIZE,
        )
        .context("failed to send RGB brightness packet")?;

        self.apply().context("failed to send RGB apply packet")
    }

    pub fn set_side_light(
        &self,
        effect: RgbSideEffect,
        color: RgbColor,
        brightness: u8,
        speed: u8,
        color_mode: RgbColorMode,
        palette_index: u8,
    ) -> Result<()> {
        let light_payload = build_side_light_payload(
            effect.protocol_id(),
            brightness,
            speed,
            color_mode,
            palette_index,
            color,
        );
        self.send_set_data(
            SUBCMD_SIDE_LIGHT,
            SIDE_LIGHT_OFFSET,
            &light_payload,
            SET_DATA_WINDOW_SIZE,
        )
        .context("failed to send side-light packet")?;

        let brightness_payload = [brightness];
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
        effect: RgbSideEffect,
        color: RgbColor,
        brightness: u8,
        speed: u8,
        color_mode: RgbColorMode,
        palette_index: u8,
        base_offset: u16,
    ) -> Result<()> {
        let light_payload = build_side_light_payload(
            effect.protocol_id(),
            brightness,
            speed,
            color_mode,
            palette_index,
            color,
        );
        self.send_set_data(
            SUBCMD_SIDE_LIGHT,
            base_offset,
            &light_payload,
            SET_DATA_WINDOW_SIZE,
        )
        .context("failed to send decorative-light packet")?;

        let brightness_payload = [brightness];
        self.send_set_data(
            SUBCMD_MAIN_BRIGHTNESS,
            base_offset + 1,
            &brightness_payload,
            SET_DATA_WINDOW_SIZE,
        )
        .context("failed to send decorative-light brightness packet")?;

        Ok(())
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

fn calc_checksum(packet: &Report) -> u8 {
    packet[4..]
        .iter()
        .copied()
        .fold(0u8, |acc, b| acc.wrapping_add(b))
}
