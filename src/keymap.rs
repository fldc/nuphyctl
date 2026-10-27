//! Keymap model: device geometry, named key positions, the keymap buffer,
//! declarative TOML configs, diffs and write planning. Nothing here touches
//! the device, so all of it is unit-tested.

use crate::keycodes::{display_keycode, parse_keycode};
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Largest payload a single `0xb2`/`0xb3` packet carries (64 - 8 header bytes).
pub const MAX_CHUNK: usize = 56;

pub const MODE_NAMES: [&str; 2] = ["mac", "windows"];
pub const LAYER_NAMES: [&str; 4] = ["base", "fn", "fn2", "fn3"];

/// Decoded `GetBase` (`0xa0`) reply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometry {
    pub current_mode: u8,
    pub layer_count: u8,
    pub layers_per_mode: u8,
    pub rows: u8,
    pub cols: u8,
    pub extras: u8,
}

impl Geometry {
    /// NuPhy Air75 V3 as reported by firmware 1.0.17.6 (`01 08 04 06 10 82`).
    pub const AIR75_V3: Self = Self {
        current_mode: 1,
        layer_count: 8,
        layers_per_mode: 4,
        rows: 6,
        cols: 16,
        extras: 2,
    };

    pub fn from_base(data: &[u8]) -> Result<Self> {
        if data.len() < 6 {
            bail!("GetBase reply too short: {} bytes", data.len());
        }
        // Same rule as NuPhy IO: bit 7 set means the low bits carry the
        // number of extra (non-matrix) entries per layer, otherwise 2.
        let extras = if data[5] & 0x80 != 0 {
            data[5] & 0x7f
        } else {
            2
        };
        let geometry = Self {
            current_mode: data[0],
            layer_count: data[1],
            layers_per_mode: data[2],
            rows: data[3],
            cols: data[4],
            extras,
        };
        if geometry.layer_count == 0 || geometry.rows == 0 || geometry.cols == 0 {
            bail!("implausible keyboard geometry from GetBase: {data:02x?}");
        }
        if geometry.total_bytes() > usize::from(u16::MAX) {
            bail!("keymap too large for 16-bit addressing: {data:02x?}");
        }
        Ok(geometry)
    }

    pub fn matrix_keys(&self) -> usize {
        usize::from(self.rows) * usize::from(self.cols)
    }

    pub fn keys_per_layer(&self) -> usize {
        self.matrix_keys() + usize::from(self.extras)
    }

    pub fn layer_bytes(&self) -> usize {
        2 * self.keys_per_layer()
    }

    pub fn total_bytes(&self) -> usize {
        usize::from(self.layer_count) * self.layer_bytes()
    }

    /// Byte address of a key entry in the device keymap.
    pub fn offset(&self, layer: u8, index: usize) -> usize {
        usize::from(layer) * self.layer_bytes() + 2 * index
    }

    pub fn check_layer(&self, layer: u8) -> Result<()> {
        if layer >= self.layer_count {
            bail!(
                "layer {layer} out of range (device has {} layers)",
                self.layer_count
            );
        }
        Ok(())
    }

    pub fn mode_count(&self) -> u8 {
        self.layer_count
            .checked_div(self.layers_per_mode)
            .unwrap_or(1)
    }

    pub fn mode_name(&self, mode: u8) -> String {
        MODE_NAMES
            .get(usize::from(mode))
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("mode{mode}"))
    }

    /// Layers belonging to a mode (`mac` = 0, `windows` = 1).
    pub fn mode_layers(&self, mode: u8) -> Result<std::ops::Range<u8>> {
        if mode >= self.mode_count() {
            bail!("mode {mode} out of range");
        }
        let start = mode * self.layers_per_mode;
        Ok(start..start + self.layers_per_mode)
    }

    /// Human label like `windows base` for a layer.
    pub fn layer_label(&self, layer: u8) -> String {
        if self.layers_per_mode == 0 {
            return format!("layer {layer}");
        }
        let mode = layer / self.layers_per_mode;
        let within = layer % self.layers_per_mode;
        let name = LAYER_NAMES
            .get(usize::from(within))
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("layer{within}"));
        format!("{} {}", self.mode_name(mode), name)
    }

    pub fn row_col(&self, index: usize) -> Option<(u8, u8)> {
        if index < self.matrix_keys() {
            let cols = usize::from(self.cols);
            Some(((index / cols) as u8, (index % cols) as u8))
        } else {
            None
        }
    }
}

pub fn parse_mode(input: &str) -> Result<u8> {
    match input.trim().to_ascii_lowercase().as_str() {
        "mac" | "macos" | "osx" => Ok(0),
        "windows" | "win" => Ok(1),
        other => bail!("unknown mode {other:?} (expected mac or windows)"),
    }
}

fn parse_layer_name(input: &str) -> Result<u8> {
    let lower = input.trim().to_ascii_lowercase();
    if let Some(i) = LAYER_NAMES.iter().position(|n| *n == lower) {
        return Ok(i as u8);
    }
    if let Some(n) = lower.strip_prefix("layer")
        && let Ok(n) = n.parse::<u8>()
    {
        return Ok(n);
    }
    bail!("unknown layer name {input:?} (expected base, fn, fn2 or fn3)")
}

/// Parse a layer: an absolute number (`5`) or `mode.name` (`windows.fn`).
pub fn parse_layer_spec(input: &str, geometry: &Geometry) -> Result<u8> {
    let trimmed = input.trim();
    let layer = if let Ok(n) = trimmed.parse::<u8>() {
        n
    } else if let Some((mode, name)) = trimmed.split_once(['.', ' ', ':']) {
        resolve_mode_layer(parse_mode(mode)?, parse_layer_name(name)?, geometry)?
    } else {
        bail!("invalid layer {input:?} (expected a number or e.g. windows.fn)");
    };
    geometry.check_layer(layer)?;
    Ok(layer)
}

fn resolve_mode_layer(mode: u8, within: u8, geometry: &Geometry) -> Result<u8> {
    if within >= geometry.layers_per_mode {
        bail!(
            "layer {within} out of range within a mode ({} layers per mode)",
            geometry.layers_per_mode
        );
    }
    let layer = geometry.mode_layers(mode)?.start + within;
    geometry.check_layer(layer)?;
    Ok(layer)
}

/// A named physical key position.
#[derive(Debug)]
pub struct NamedPosition {
    pub name: &'static str,
    pub index: usize,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
}

macro_rules! pos {
    ($name:expr, $row:expr, $col:expr) => {
        pos!($name, $row, $col, [], "")
    };
    ($name:expr, $row:expr, $col:expr, [$($alias:expr),*], $desc:expr) => {
        NamedPosition {
            name: $name,
            index: $row * 16 + $col,
            aliases: &[$($alias),*],
            description: $desc,
        }
    };
}

/// Air75 V3 key positions, named after the Windows-mode legends.
pub static AIR75_V3_POSITIONS: &[NamedPosition] = &[
    pos!("ESC", 0, 0, ["ESCAPE"], ""),
    pos!("F1", 0, 1),
    pos!("F2", 0, 2),
    pos!("F3", 0, 3),
    pos!("F4", 0, 4),
    pos!("F5", 0, 5),
    pos!("F6", 0, 6),
    pos!("F7", 0, 7),
    pos!("F8", 0, 8),
    pos!("F9", 0, 9),
    pos!("F10", 0, 10),
    pos!("F11", 0, 11),
    pos!("F12", 0, 12),
    pos!(
        "PSCR",
        0,
        13,
        ["PRINT_SCREEN", "PRTSC"],
        "factory: area screenshot"
    ),
    pos!(
        "YELLOW",
        0,
        14,
        ["INS", "INSERT"],
        "yellow key, factory Insert"
    ),
    pos!("GRV", 1, 0, ["GRAVE"], ""),
    pos!("1", 1, 1),
    pos!("2", 1, 2),
    pos!("3", 1, 3),
    pos!("4", 1, 4),
    pos!("5", 1, 5),
    pos!("6", 1, 6),
    pos!("7", 1, 7),
    pos!("8", 1, 8),
    pos!("9", 1, 9),
    pos!("0", 1, 10),
    pos!("MINS", 1, 11, ["MINUS"], ""),
    pos!("EQL", 1, 12, ["EQUAL"], ""),
    pos!("BSPC", 1, 13, ["BACKSPACE"], ""),
    pos!("PGUP", 1, 15, ["PAGE_UP"], ""),
    pos!("TAB", 2, 0),
    pos!("Q", 2, 1),
    pos!("W", 2, 2),
    pos!("E", 2, 3),
    pos!("R", 2, 4),
    pos!("T", 2, 5),
    pos!("Y", 2, 6),
    pos!("U", 2, 7),
    pos!("I", 2, 8),
    pos!("O", 2, 9),
    pos!("P", 2, 10),
    pos!("LBRC", 2, 11, ["LEFT_BRACKET", "LBRACKET"], ""),
    pos!("RBRC", 2, 12, ["RIGHT_BRACKET", "RBRACKET"], ""),
    pos!("BSLS", 2, 13, ["BACKSLASH"], ""),
    pos!("PGDN", 2, 15, ["PAGE_DOWN"], ""),
    pos!("CAPS", 3, 0, ["CAPS_LOCK"], ""),
    pos!("A", 3, 1),
    pos!("S", 3, 2),
    pos!("D", 3, 3),
    pos!("F", 3, 4),
    pos!("G", 3, 5),
    pos!("H", 3, 6),
    pos!("J", 3, 7),
    pos!("K", 3, 8),
    pos!("L", 3, 9),
    pos!("SCLN", 3, 10, ["SEMICOLON"], ""),
    pos!("QUOT", 3, 11, ["QUOTE"], ""),
    pos!(
        "KNOB",
        3,
        12,
        ["RED", "KNOB_PRESS", "MODULE"],
        "red top-right key / knob module press, factory Mute"
    ),
    pos!("ENT", 3, 13, ["ENTER"], ""),
    pos!("HOME", 3, 15),
    pos!("LSFT", 4, 0, ["LSHIFT", "LEFT_SHIFT"], ""),
    pos!("Z", 4, 2),
    pos!("X", 4, 3),
    pos!("C", 4, 4),
    pos!("V", 4, 5),
    pos!("B", 4, 6),
    pos!("N", 4, 7),
    pos!("M", 4, 8),
    pos!("COMM", 4, 9, ["COMMA"], ""),
    pos!("DOT", 4, 10),
    pos!("SLSH", 4, 11, ["SLASH"], ""),
    pos!("RSFT", 4, 12, ["RSHIFT", "RIGHT_SHIFT"], ""),
    pos!("UP", 4, 14),
    pos!("END", 4, 15),
    pos!("LCTL", 5, 0, ["LCTRL", "LEFT_CTRL"], ""),
    pos!(
        "LGUI",
        5,
        1,
        ["LWIN", "LOPT"],
        "Win in Windows mode, Option in Mac mode"
    ),
    pos!(
        "LALT",
        5,
        2,
        ["LCMD"],
        "Alt in Windows mode, Cmd in Mac mode"
    ),
    pos!("SPC", 5, 6, ["SPACE"], ""),
    pos!(
        "RALT",
        5,
        10,
        ["RCMD"],
        "Alt in Windows mode, Cmd in Mac mode"
    ),
    pos!("FN", 5, 11, [], "factory MO(1)/MO(5)"),
    pos!("RCTL", 5, 12, ["RCTRL", "RIGHT_CTRL"], ""),
    pos!("LEFT", 5, 13),
    pos!("DOWN", 5, 14),
    pos!("RGHT", 5, 15, ["RIGHT"], ""),
    NamedPosition {
        name: "KNOB_CCW",
        index: 96,
        aliases: &["CCW"],
        description: "knob turned counter-clockwise, factory Volume Down",
    },
    NamedPosition {
        name: "KNOB_CW",
        index: 97,
        aliases: &["CW"],
        description: "knob turned clockwise, factory Volume Up",
    },
];

/// Resolves key positions for one keyboard.
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub geometry: Geometry,
    named: &'static [NamedPosition],
}

impl Layout {
    /// Named positions are used when the matrix matches the Air75 V3.
    pub fn new(geometry: Geometry) -> Self {
        let named: &'static [NamedPosition] =
            if geometry.rows == 6 && geometry.cols == 16 && geometry.extras == 2 {
                AIR75_V3_POSITIONS
            } else {
                &[]
            };
        Self { geometry, named }
    }

    pub fn has_names(&self) -> bool {
        !self.named.is_empty()
    }

    pub fn name_of(&self, index: usize) -> Option<&'static str> {
        self.named.iter().find(|p| p.index == index).map(|p| p.name)
    }

    /// Parse a position: a name (`F1`, `KNOB_CCW`, `YELLOW`), `row,col`, or
    /// `#index` (entry index within a layer; extras follow the matrix).
    pub fn parse_position(&self, input: &str) -> Result<usize> {
        let trimmed = input.trim();
        let keys = self.geometry.keys_per_layer();

        if let Some(idx) = trimmed.strip_prefix('#') {
            let index: usize = idx
                .trim()
                .parse()
                .with_context(|| format!("invalid position index {trimmed:?}"))?;
            if index >= keys {
                bail!("position index {index} out of range (0..{keys})");
            }
            return Ok(index);
        }

        if let Some((r, c)) = trimmed.split_once(',') {
            let row: u8 = r
                .trim()
                .parse()
                .with_context(|| format!("invalid row in {trimmed:?}"))?;
            let col: u8 = c
                .trim()
                .parse()
                .with_context(|| format!("invalid column in {trimmed:?}"))?;
            if row >= self.geometry.rows || col >= self.geometry.cols {
                bail!(
                    "position {row},{col} outside the {}x{} matrix",
                    self.geometry.rows,
                    self.geometry.cols
                );
            }
            return Ok(usize::from(row) * usize::from(self.geometry.cols) + usize::from(col));
        }

        let upper = trimmed.to_ascii_uppercase();
        let upper = upper.strip_prefix("KC_").unwrap_or(&upper);
        if let Some(p) = self
            .named
            .iter()
            .find(|p| p.name == upper || p.aliases.contains(&upper))
        {
            return Ok(p.index);
        }

        // Knob extras exist on any board reporting at least two extras.
        let extra = self.geometry.matrix_keys();
        match upper {
            "KNOB_CCW" | "CCW" if self.geometry.extras >= 1 => return Ok(extra),
            "KNOB_CW" | "CW" if self.geometry.extras >= 2 => return Ok(extra + 1),
            _ => {}
        }

        if self.has_names() {
            bail!(
                "unknown key position {trimmed:?}; see `nuphyctl keymap positions`, or use row,col or #index"
            )
        }
        bail!(
            "unknown key position {trimmed:?}; this keyboard's layout has no named positions, use row,col or #index"
        )
    }

    /// Short label for a position: name when known, else `r,c` / `#index`.
    pub fn label(&self, index: usize) -> String {
        if let Some(name) = self.name_of(index) {
            return name.to_string();
        }
        match self.geometry.row_col(index) {
            Some((r, c)) => format!("{r},{c}"),
            None => format!("#{index}"),
        }
    }
}

/// A full keymap buffer as read with `GetUseKeys`/`GetDefaultKeys`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keymap {
    pub geometry: Geometry,
    pub bytes: Vec<u8>,
}

impl Keymap {
    pub fn from_bytes(geometry: Geometry, bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() != geometry.total_bytes() {
            bail!(
                "keymap is {} bytes, expected {} ({} layers x {} bytes)",
                bytes.len(),
                geometry.total_bytes(),
                geometry.layer_count,
                geometry.layer_bytes()
            );
        }
        Ok(Self { geometry, bytes })
    }

    pub fn get(&self, layer: u8, index: usize) -> u16 {
        let off = self.geometry.offset(layer, index);
        u16::from_be_bytes([self.bytes[off], self.bytes[off + 1]])
    }

    pub fn set(&mut self, layer: u8, index: usize, code: u16) {
        let off = self.geometry.offset(layer, index);
        self.bytes[off..off + 2].copy_from_slice(&code.to_be_bytes());
    }
}

/// One key assignment requested by the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assignment {
    pub layer: u8,
    pub index: usize,
    pub code: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub layer: u8,
    pub index: usize,
    pub from: u16,
    pub to: u16,
}

impl Change {
    pub fn describe(&self, layout: &Layout) -> String {
        let pos = match layout.geometry.row_col(self.index) {
            Some((r, c)) => format!("{} ({r},{c})", layout.label(self.index)),
            None => format!("{} (#{})", layout.label(self.index), self.index),
        };
        format!(
            "layer {} ({}) {}: {} (0x{:04x}) -> {} (0x{:04x})",
            self.layer,
            layout.geometry.layer_label(self.layer),
            pos,
            display_keycode(self.from),
            self.from,
            display_keycode(self.to),
            self.to
        )
    }
}

/// Changes needed to turn `current` into the requested assignments.
pub fn diff_assignments(current: &Keymap, assignments: &[Assignment]) -> Vec<Change> {
    assignments
        .iter()
        .filter_map(|a| {
            let from = current.get(a.layer, a.index);
            (from != a.code).then_some(Change {
                layer: a.layer,
                index: a.index,
                from,
                to: a.code,
            })
        })
        .collect()
}

/// All entries that differ between two keymaps of the same geometry.
pub fn diff_keymaps(current: &Keymap, desired: &Keymap) -> Vec<Change> {
    let g = current.geometry;
    let mut out = Vec::new();
    for layer in 0..g.layer_count {
        for index in 0..g.keys_per_layer() {
            let from = current.get(layer, index);
            let to = desired.get(layer, index);
            if from != to {
                out.push(Change {
                    layer,
                    index,
                    from,
                    to,
                });
            }
        }
    }
    out
}

pub fn apply_changes(keymap: &Keymap, changes: &[Change]) -> Keymap {
    let mut out = keymap.clone();
    for c in changes {
        out.set(c.layer, c.index, c.to);
    }
    out
}

/// A `SetUseKeys` write: byte address plus payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteChunk {
    pub addr: u16,
    pub data: Vec<u8>,
}

/// Group changed entries into writes. Only changed entries are written;
/// adjacent ones share a packet (up to `MAX_CHUNK` bytes).
pub fn plan_writes(geometry: &Geometry, changes: &[Change]) -> Vec<WriteChunk> {
    let mut entries: Vec<(usize, u16)> = changes
        .iter()
        .map(|c| (geometry.offset(c.layer, c.index), c.to))
        .collect();
    entries.sort_by_key(|(off, _)| *off);
    entries.dedup_by_key(|(off, _)| *off);

    let mut chunks: Vec<WriteChunk> = Vec::new();
    let mut next_off = usize::MAX;
    for (off, code) in entries {
        match chunks.last_mut() {
            Some(chunk) if off == next_off && chunk.data.len() + 2 <= MAX_CHUNK => {
                chunk.data.extend_from_slice(&code.to_be_bytes());
            }
            _ => chunks.push(WriteChunk {
                addr: off as u16,
                data: code.to_be_bytes().to_vec(),
            }),
        }
        next_off = off + 2;
    }
    chunks
}

/// Parse a declarative keymap file. Layout:
///
/// ```toml
/// [windows.base]        # mac|windows . base|fn|fn2|fn3
/// F1 = "BRID"
/// "3,12" = "MUTE"
/// [windows.base.knob]   # ccw / cw / press
/// ccw = "VOLD"
/// [layers.6]            # absolute layer number
/// ESC = 0x29
/// ```
pub fn parse_config(text: &str, layout: &Layout) -> Result<Vec<Assignment>> {
    let table: toml::Table = text.parse().context("invalid TOML")?;
    let geometry = &layout.geometry;
    let mut out: BTreeMap<(u8, usize), (u16, String)> = BTreeMap::new();

    for (top, value) in &table {
        let section = value
            .as_table()
            .with_context(|| format!("[{top}] must be a table"))?;
        match top.as_str() {
            "layers" | "layer" => {
                for (layer_key, keys) in section {
                    let layer: u8 = layer_key
                        .parse()
                        .with_context(|| format!("invalid layer number [{top}.{layer_key}]"))?;
                    geometry.check_layer(layer)?;
                    let keys = keys
                        .as_table()
                        .with_context(|| format!("[{top}.{layer_key}] must be a table"))?;
                    collect_layer(layout, layer, &format!("{top}.{layer_key}"), keys, &mut out)?;
                }
            }
            mode_name => {
                let mode = parse_mode(mode_name).with_context(|| {
                    format!("unknown section [{top}] (expected mac, windows or layers)")
                })?;
                for (layer_key, keys) in section {
                    let within = parse_layer_name(layer_key)
                        .with_context(|| format!("in [{top}.{layer_key}]"))?;
                    let layer = resolve_mode_layer(mode, within, geometry)
                        .with_context(|| format!("in [{top}.{layer_key}]"))?;
                    let keys = keys
                        .as_table()
                        .with_context(|| format!("[{top}.{layer_key}] must be a table"))?;
                    collect_layer(layout, layer, &format!("{top}.{layer_key}"), keys, &mut out)?;
                }
            }
        }
    }

    Ok(out
        .into_iter()
        .map(|((layer, index), (code, _))| Assignment { layer, index, code })
        .collect())
}

fn collect_layer(
    layout: &Layout,
    layer: u8,
    section: &str,
    keys: &toml::Table,
    out: &mut BTreeMap<(u8, usize), (u16, String)>,
) -> Result<()> {
    for (key, value) in keys {
        if key.eq_ignore_ascii_case("knob") && value.is_table() {
            let knob = value.as_table().expect("checked is_table");
            for (dir, value) in knob {
                let pos = match dir.to_ascii_lowercase().as_str() {
                    "ccw" => "KNOB_CCW",
                    "cw" => "KNOB_CW",
                    "press" => "KNOB",
                    other => bail!(
                        "unknown knob entry {other:?} in [{section}.knob] (expected ccw, cw or press)"
                    ),
                };
                let source = format!("{section}.knob.{dir}");
                insert_assignment(layout, layer, pos, value, &source, out)?;
            }
            continue;
        }
        let source = format!("{section}.{key}");
        insert_assignment(layout, layer, key, value, &source, out)?;
    }
    Ok(())
}

fn insert_assignment(
    layout: &Layout,
    layer: u8,
    position: &str,
    value: &toml::Value,
    source: &str,
    out: &mut BTreeMap<(u8, usize), (u16, String)>,
) -> Result<()> {
    let index = layout
        .parse_position(position)
        .with_context(|| format!("in {source}"))?;
    let code = match value {
        toml::Value::String(s) => parse_keycode(s).with_context(|| format!("in {source}"))?,
        toml::Value::Integer(n) => u16::try_from(*n)
            .ok()
            .with_context(|| format!("keycode {n} out of range in {source}"))?,
        other => bail!(
            "{source} must be a keycode string or integer, got {}",
            other.type_str()
        ),
    };
    if let Some((_, previous)) = out.get(&(layer, index)) {
        bail!("{source} assigns the same key as {previous}");
    }
    out.insert((layer, index), (code, source.to_string()));
    Ok(())
}

/// Render the keys of `current` that differ from `defaults` as a config file
/// that `parse_config` reads back.
pub fn export_config(current: &Keymap, defaults: &Keymap, layout: &Layout) -> String {
    let g = current.geometry;
    let mut out = String::from(
        "# nuphyctl keymap: keys that differ from the factory keymap\n# apply with: nuphyctl keymap apply <this file>\n",
    );
    for layer in 0..g.layer_count {
        let changed: Vec<usize> = (0..g.keys_per_layer())
            .filter(|&i| current.get(layer, i) != defaults.get(layer, i))
            .collect();
        if changed.is_empty() {
            continue;
        }
        let header = if g.layers_per_mode > 0
            && usize::from(layer / g.layers_per_mode) < MODE_NAMES.len()
            && usize::from(layer % g.layers_per_mode) < LAYER_NAMES.len()
        {
            format!(
                "{}.{}",
                MODE_NAMES[usize::from(layer / g.layers_per_mode)],
                LAYER_NAMES[usize::from(layer % g.layers_per_mode)]
            )
        } else {
            format!("layers.{layer}")
        };
        out.push_str(&format!("\n[{header}]  # layer {layer}\n"));
        for index in changed {
            let label = layout.label(index);
            let key = if label.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && !label.chars().all(|c| c.is_ascii_digit())
            {
                label
            } else {
                format!("\"{label}\"")
            };
            out.push_str(&format!(
                "{key} = \"{}\"  # factory {}\n",
                display_keycode(current.get(layer, index)),
                display_keycode(defaults.get(layer, index))
            ));
        }
    }
    out
}

/// `$XDG_STATE_HOME/nuphyctl`, falling back to `~/.local/state/nuphyctl`.
pub fn default_backup_dir() -> Result<PathBuf> {
    if let Some(state) = std::env::var_os("XDG_STATE_HOME").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(state).join("nuphyctl"));
    }
    let home = std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .context("neither XDG_STATE_HOME nor HOME is set; cannot choose a backup directory")?;
    Ok(PathBuf::from(home).join(".local/state/nuphyctl"))
}

/// Write a backup as `keymap-YYYYMMDDTHHMMSSZ.bin` and return its path.
pub fn write_backup(dir: &Path, bytes: &[u8]) -> Result<PathBuf> {
    std::fs::create_dir_all(dir)
        .with_context(|| format!("failed to create backup directory {}", dir.display()))?;
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let stamp = utc_timestamp(secs);
    for n in 0..1000 {
        let name = if n == 0 {
            format!("keymap-{stamp}.bin")
        } else {
            format!("keymap-{stamp}-{n}.bin")
        };
        let path = dir.join(name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                use std::io::Write;
                f.write_all(bytes)
                    .with_context(|| format!("failed to write backup {}", path.display()))?;
                f.sync_all()
                    .with_context(|| format!("failed to sync backup {}", path.display()))?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => {
                return Err(e)
                    .with_context(|| format!("failed to create backup {}", path.display()));
            }
        }
    }
    bail!(
        "could not find a free backup file name in {}",
        dir.display()
    )
}

/// `YYYYMMDDTHHMMSSZ` for a Unix timestamp (UTC).
pub fn utc_timestamp(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}{month:02}{day:02}T{:02}{:02}{:02}Z",
        rem / 3600,
        (rem / 60) % 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        Layout::new(Geometry::AIR75_V3)
    }

    fn blank() -> Keymap {
        let g = Geometry::AIR75_V3;
        Keymap::from_bytes(g, vec![0; g.total_bytes()]).unwrap()
    }

    #[test]
    fn decodes_get_base() {
        let g = Geometry::from_base(&[0x01, 0x08, 0x04, 0x06, 0x10, 0x82, 0, 0]).unwrap();
        assert_eq!(g, Geometry::AIR75_V3);
        assert_eq!(g.layer_bytes(), 196);
        assert_eq!(g.total_bytes(), 1568);
        let g = Geometry::from_base(&[0, 8, 4, 6, 16, 0]).unwrap();
        assert_eq!(g.extras, 2);
        assert!(Geometry::from_base(&[0, 0, 4, 6, 16, 0x82]).is_err());
        assert!(Geometry::from_base(&[0, 8]).is_err());
    }

    #[test]
    fn offsets() {
        let g = Geometry::AIR75_V3;
        assert_eq!(g.offset(0, 0), 0);
        assert_eq!(g.offset(4, 1), 4 * 196 + 2);
        assert_eq!(g.offset(5, 96), 5 * 196 + 192);
        assert_eq!(g.offset(7, 97), 7 * 196 + 194);
        assert_eq!(g.row_col(60), Some((3, 12)));
        assert_eq!(g.row_col(96), None);
        assert_eq!(g.mode_layers(1).unwrap(), 4..8);
        assert_eq!(g.mode_layers(0).unwrap(), 0..4);
        assert!(g.mode_layers(2).is_err());
        assert_eq!(g.layer_label(5), "windows fn");
        assert_eq!(g.layer_label(2), "mac fn2");
    }

    #[test]
    fn parses_layers() {
        let g = Geometry::AIR75_V3;
        assert_eq!(parse_layer_spec("4", &g).unwrap(), 4);
        assert_eq!(parse_layer_spec("windows.base", &g).unwrap(), 4);
        assert_eq!(parse_layer_spec("windows.fn", &g).unwrap(), 5);
        assert_eq!(parse_layer_spec("mac.fn3", &g).unwrap(), 3);
        assert_eq!(parse_layer_spec("win.layer2", &g).unwrap(), 6);
        assert!(parse_layer_spec("8", &g).is_err());
        assert!(parse_layer_spec("linux.base", &g).is_err());
        assert!(parse_layer_spec("windows.fn4", &g).is_err());
    }

    #[test]
    fn parses_positions() {
        let l = layout();
        assert_eq!(l.parse_position("ESC").unwrap(), 0);
        assert_eq!(l.parse_position("f1").unwrap(), 1);
        assert_eq!(l.parse_position("KC_F12").unwrap(), 12);
        assert_eq!(l.parse_position("PSCR").unwrap(), 13);
        assert_eq!(l.parse_position("YELLOW").unwrap(), 14);
        assert_eq!(l.parse_position("INSERT").unwrap(), 14);
        assert_eq!(l.parse_position("KNOB").unwrap(), 60);
        assert_eq!(l.parse_position("KNOB_CCW").unwrap(), 96);
        assert_eq!(l.parse_position("knob_cw").unwrap(), 97);
        assert_eq!(l.parse_position("1").unwrap(), 17);
        assert_eq!(l.parse_position("0").unwrap(), 26);
        assert_eq!(l.parse_position("FN").unwrap(), 91);
        assert_eq!(l.parse_position("RIGHT").unwrap(), 95);
        assert_eq!(l.parse_position("3,12").unwrap(), 60);
        assert_eq!(l.parse_position(" 0 , 15 ").unwrap(), 15);
        assert_eq!(l.parse_position("#17").unwrap(), 17);
        assert_eq!(l.parse_position("#97").unwrap(), 97);
        assert!(l.parse_position("#98").is_err());
        assert!(l.parse_position("6,0").is_err());
        assert!(l.parse_position("0,16").is_err());
        assert!(l.parse_position("NOPE").is_err());
    }

    #[test]
    fn position_names_are_unique_and_in_range() {
        let mut seen = std::collections::HashSet::new();
        let mut indices = std::collections::HashSet::new();
        for p in AIR75_V3_POSITIONS {
            assert!(p.index < Geometry::AIR75_V3.keys_per_layer());
            assert!(indices.insert(p.index), "duplicate index {}", p.index);
            for name in std::iter::once(&p.name).chain(p.aliases.iter()) {
                assert!(seen.insert(*name), "duplicate position name {name}");
            }
        }
    }

    #[test]
    fn unnamed_layout_still_accepts_row_col_and_knob() {
        let g = Geometry {
            rows: 5,
            cols: 15,
            ..Geometry::AIR75_V3
        };
        let l = Layout::new(g);
        assert!(!l.has_names());
        assert_eq!(l.parse_position("1,2").unwrap(), 17);
        assert_eq!(l.parse_position("KNOB_CW").unwrap(), 76);
        assert!(l.parse_position("ESC").is_err());
        assert_eq!(l.label(17), "1,2");
        assert_eq!(l.label(76), "#76");
    }

    #[test]
    fn keymap_get_set_big_endian() {
        let mut k = blank();
        k.set(4, 1, 0x00be);
        k.set(5, 97, 0x7e06);
        assert_eq!(&k.bytes[4 * 196 + 2..4 * 196 + 4], &[0x00, 0xbe]);
        assert_eq!(k.get(4, 1), 0x00be);
        assert_eq!(k.get(5, 97), 0x7e06);
        assert!(Keymap::from_bytes(Geometry::AIR75_V3, vec![0; 100]).is_err());
    }

    const SAMPLE: &str = r##"
        [windows.base]   # layer 4
        F1 = "BRID"
        F3 = "MISSION_CONTROL"
        "3,12" = "MUTE"
        "#14" = 0x49

        [windows.base.knob]
        ccw = "VOLD"
        cw = "kc_volu"

        [windows.fn]     # layer 5
        F1 = "F1"
        FN = "MO(5)"

        [layers.2]
        ESC = "TRNS"
    "##;

    #[test]
    fn parses_config() {
        let a = parse_config(SAMPLE, &layout()).unwrap();
        let got: Vec<(u8, usize, u16)> = a.iter().map(|a| (a.layer, a.index, a.code)).collect();
        assert_eq!(
            got,
            vec![
                (2, 0, 0x0001),
                (4, 1, 0x00be),
                (4, 3, 0x7e06),
                (4, 14, 0x0049),
                (4, 60, 0x00a8),
                (4, 96, 0x00aa),
                (4, 97, 0x00a9),
                (5, 1, 0x003a),
                (5, 91, 0x5225),
            ]
        );
    }

    #[test]
    fn config_errors() {
        let l = layout();
        assert!(parse_config("[linux.base]\nF1 = \"F1\"", &l).is_err());
        assert!(parse_config("[windows.fn9]\nF1 = \"F1\"", &l).is_err());
        assert!(parse_config("[windows.base]\nNOPE = \"F1\"", &l).is_err());
        assert!(parse_config("[windows.base]\nF1 = \"NOPE\"", &l).is_err());
        assert!(parse_config("[windows.base]\nF1 = 70000", &l).is_err());
        assert!(parse_config("[windows.base]\nF1 = true", &l).is_err());
        assert!(parse_config("[layers.8]\nF1 = \"F1\"", &l).is_err());
        assert!(parse_config("[windows.base.knob]\nleft = \"F1\"", &l).is_err());
        let dup = "[windows.base]\nF1 = \"F1\"\n\"0,1\" = \"F2\"";
        let err = parse_config(dup, &l).unwrap_err();
        assert!(format!("{err:#}").contains("same key"), "{err:#}");
        let dup = "[windows.base]\nKNOB_CW = \"F1\"\n[windows.base.knob]\ncw = \"F1\"";
        assert!(parse_config(dup, &l).is_err());
        let dup = "[windows.base]\nESC = \"F1\"\n[layers.4]\nESC = \"F1\"";
        assert!(parse_config(dup, &l).is_err());
    }

    #[test]
    fn diff_and_idempotence() {
        let current = blank();
        let assignments = parse_config(SAMPLE, &layout()).unwrap();
        let changes = diff_assignments(&current, &assignments);
        assert_eq!(changes.len(), assignments.len());
        assert!(changes.iter().all(|c| c.from == 0));

        let desired = apply_changes(&current, &changes);
        assert!(diff_assignments(&desired, &assignments).is_empty());
        assert_eq!(diff_keymaps(&current, &desired), changes);
        assert!(diff_keymaps(&desired, &desired).is_empty());
    }

    #[test]
    fn diff_skips_equal_values() {
        let mut current = blank();
        current.set(4, 1, 0x00be);
        let a = vec![
            Assignment {
                layer: 4,
                index: 1,
                code: 0x00be,
            },
            Assignment {
                layer: 4,
                index: 2,
                code: 0x00bd,
            },
        ];
        let changes = diff_assignments(&current, &a);
        assert_eq!(
            changes,
            vec![Change {
                layer: 4,
                index: 2,
                from: 0,
                to: 0x00bd
            }]
        );
        let text = changes[0].describe(&layout());
        assert_eq!(
            text,
            "layer 4 (windows base) F2 (0,2): NO (0x0000) -> BRIU (0x00bd)"
        );
    }

    #[test]
    fn plans_coalesced_writes() {
        let g = Geometry::AIR75_V3;
        let change = |layer, index, to| Change {
            layer,
            index,
            from: 0,
            to,
        };
        let changes = vec![
            change(4, 2, 0x00bd),
            change(4, 1, 0x00be),
            change(4, 5, 0x006f),
            change(5, 0, 0x0001),
        ];
        let plan = plan_writes(&g, &changes);
        assert_eq!(
            plan,
            vec![
                WriteChunk {
                    addr: 786,
                    data: vec![0x00, 0xbe, 0x00, 0xbd]
                },
                WriteChunk {
                    addr: 794,
                    data: vec![0x00, 0x6f]
                },
                WriteChunk {
                    addr: 980,
                    data: vec![0x00, 0x01]
                },
            ]
        );

        // A whole changed layer splits into packets of at most MAX_CHUNK bytes.
        let all: Vec<Change> = (0..g.keys_per_layer())
            .map(|i| change(0, i, 0x0004))
            .collect();
        let plan = plan_writes(&g, &all);
        assert!(plan.iter().all(|c| c.data.len() <= MAX_CHUNK));
        assert_eq!(plan.iter().map(|c| c.data.len()).sum::<usize>(), 196);
        assert_eq!(plan[1].addr, 56);
    }

    #[test]
    fn export_round_trips() {
        let defaults = blank();
        let mut current = blank();
        current.set(4, 1, 0x00be);
        current.set(4, 17, 0x7e03);
        current.set(4, 60, 0x00a8);
        current.set(5, 96, 0x00aa);
        let l = layout();
        let text = export_config(&current, &defaults, &l);
        assert!(text.contains("[windows.base]"), "{text}");
        assert!(text.contains("\"1\" = \"BT1\""), "{text}");
        let a = parse_config(&text, &l).unwrap();
        let rebuilt = apply_changes(&defaults, &diff_assignments(&defaults, &a));
        assert_eq!(rebuilt, current);
    }

    #[test]
    fn timestamps() {
        assert_eq!(utc_timestamp(0), "19700101T000000Z");
        assert_eq!(utc_timestamp(951_782_400), "20000229T000000Z");
        assert_eq!(utc_timestamp(1_791_990_000), "20261014T150000Z");
    }

    #[test]
    fn backups_never_overwrite() {
        let dir = std::env::temp_dir().join(format!("nuphyctl-test-{}", std::process::id()));
        let a = write_backup(&dir, &[1, 2, 3]).unwrap();
        let b = write_backup(&dir, &[4, 5, 6]).unwrap();
        assert_ne!(a, b);
        assert_eq!(std::fs::read(&a).unwrap(), vec![1, 2, 3]);
        assert_eq!(std::fs::read(&b).unwrap(), vec![4, 5, 6]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
