//! `nuphyctl keymap ...` command handlers.

use crate::app::is_retryable_rgb_error;
use crate::cli::{
    DeviceSelector, KeymapApplyArgs, KeymapDumpArgs, KeymapExportArgs, KeymapInfoArgs,
    KeymapListArgs, KeymapResetArgs, KeymapResetMode, KeymapRestoreBackupArgs, KeymapSetArgs,
};
use crate::hid_transport::{clear_input_reports, open_selected_device};
use crate::keycodes::{KEYCODES, format_hex, keycode_name, parse_keycode};
use crate::keymap::{
    AIR75_V3_POSITIONS, Assignment, Change, Geometry, Keymap, Layout, apply_changes,
    default_backup_dir, diff_assignments, diff_keymaps, export_config, parse_config,
    parse_layer_spec, plan_writes, write_backup,
};
use crate::nuphy_protocol::{KeyboardProtocol, format_firmware_version};
use anyhow::{Context, Result, bail};
use hidapi::HidApi;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::{thread, time::Duration};

const CONNECT_MAX_ATTEMPTS: usize = 4;
const CONNECT_RETRY_BASE_DELAY_MS: u64 = 120;

struct Keyboard<'a> {
    protocol: KeyboardProtocol<'a>,
    layout: Layout,
    product: Option<String>,
}

impl Keyboard<'_> {
    fn geometry(&self) -> Geometry {
        self.layout.geometry
    }

    fn read(&self, factory: bool) -> Result<Keymap> {
        let g = self.geometry();
        let bytes = self.protocol.read_keymap(factory, g.total_bytes())?;
        Keymap::from_bytes(g, bytes)
    }

    /// Back up the current keymap, write `changes`, then read everything back
    /// and require it to equal `current` with `changes` applied.
    fn write_and_verify(&self, current: &Keymap, changes: &[Change]) -> Result<PathBuf> {
        let backup = backup_keymap(current)?;
        eprintln!("backup: {}", backup.display());
        let desired = apply_changes(current, changes);
        for chunk in plan_writes(&self.geometry(), changes) {
            self.protocol
                .write_keymap(chunk.addr, &chunk.data)
                .with_context(|| {
                    format!(
                        "write failed; restore with: nuphyctl keymap restore-backup {}",
                        backup.display()
                    )
                })?;
        }
        self.verify(&desired, &backup)?;
        Ok(backup)
    }

    fn verify(&self, desired: &Keymap, backup: &Path) -> Result<()> {
        let after = self
            .read(false)
            .context("failed to read keymap back for verification")?;
        let mismatches = diff_keymaps(desired, &after);
        if !mismatches.is_empty() {
            let lines: Vec<String> = mismatches
                .iter()
                .map(|c| {
                    format!(
                        "  layer {} {}: expected {}, device has {}",
                        c.layer,
                        self.layout.label(c.index),
                        format_hex(c.from),
                        format_hex(c.to)
                    )
                })
                .collect();
            bail!(
                "verification failed, {} key(s) differ from what was expected:\n{}\nbackup of the previous keymap: {}",
                mismatches.len(),
                lines.join("\n"),
                backup.display()
            );
        }
        Ok(())
    }
}

/// Open the keyboard, negotiate a session key, read its geometry and run `f`.
/// Only connecting is retried; `f` runs once.
fn with_keyboard<T>(
    api: &HidApi,
    selector: &DeviceSelector,
    f: impl FnOnce(&Keyboard) -> Result<T>,
) -> Result<T> {
    let mut attempt = 0;
    loop {
        attempt += 1;
        let connected = open_selected_device(api, selector).and_then(|dev| {
            clear_input_reports(&dev)?;
            Ok(dev)
        });
        let err = match connected {
            Ok(dev) => match KeyboardProtocol::new(&dev) {
                Ok(protocol) => {
                    let geometry = Geometry::from_base(&protocol.get_base()?)?;
                    let product = dev.get_product_string().ok().flatten();
                    let keyboard = Keyboard {
                        protocol,
                        layout: Layout::new(geometry),
                        product,
                    };
                    return f(&keyboard);
                }
                Err(err) => err,
            },
            Err(err) => err,
        };
        if !is_retryable_rgb_error(&err) || attempt >= CONNECT_MAX_ATTEMPTS {
            return Err(err);
        }
        eprintln!(
            "transient HID error (attempt {}/{}): {} -- retrying",
            attempt, CONNECT_MAX_ATTEMPTS, err
        );
        thread::sleep(Duration::from_millis(
            CONNECT_RETRY_BASE_DELAY_MS * attempt as u64,
        ));
    }
}

fn backup_keymap(keymap: &Keymap) -> Result<PathBuf> {
    let dir = default_backup_dir()?;
    write_backup(&dir, &keymap.bytes).context("failed to save keymap backup; nothing was written")
}

fn print_json(value: &Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn code_json(code: u16) -> Value {
    json!({ "code": format_hex(code), "value": code, "name": keycode_name(code) })
}

fn position_json(layout: &Layout, layer: u8, index: usize) -> Value {
    let (row, col) = match layout.geometry.row_col(index) {
        Some((r, c)) => (json!(r), json!(c)),
        None => (Value::Null, Value::Null),
    };
    json!({
        "layer": layer,
        "layer_name": layout.geometry.layer_label(layer),
        "index": index,
        "row": row,
        "col": col,
        "position": layout.name_of(index),
    })
}

fn change_json(layout: &Layout, change: &Change) -> Value {
    let mut v = position_json(layout, change.layer, change.index);
    v["from"] = code_json(change.from);
    v["to"] = code_json(change.to);
    v
}

fn print_changes(layout: &Layout, changes: &[Change]) {
    for c in changes {
        println!("{}", c.describe(layout));
    }
}

pub fn run_info(api: &HidApi, args: &KeymapInfoArgs) -> Result<()> {
    with_keyboard(api, &args.device, |kb| {
        let fw = kb.protocol.get_firmware_info()?;
        let version = format_firmware_version(&fw);
        let g = kb.geometry();
        let modes: Vec<Value> = (0..g.mode_count())
            .map(|m| {
                let layers = g
                    .mode_layers(m)
                    .map(|r| r.collect::<Vec<u8>>())
                    .unwrap_or_default();
                json!({ "mode": m, "name": g.mode_name(m), "layers": layers })
            })
            .collect();
        if args.json {
            return print_json(&json!({
                "product": kb.product,
                "firmware_version": version,
                "session_key": format!("0x{:02x}", kb.protocol.session_key().value()),
                "current_mode": { "mode": g.current_mode, "name": g.mode_name(g.current_mode) },
                "layer_count": g.layer_count,
                "layers_per_mode": g.layers_per_mode,
                "modes": modes,
                "matrix": { "rows": g.rows, "cols": g.cols },
                "extra_entries": g.extras,
                "layer_bytes": g.layer_bytes(),
                "keymap_bytes": g.total_bytes(),
                "named_positions": kb.layout.has_names(),
            }));
        }
        println!("product: {}", kb.product.as_deref().unwrap_or("unknown"));
        println!("firmware: {version}");
        println!(
            "current mode: {} ({})",
            g.mode_name(g.current_mode),
            g.current_mode
        );
        println!("layers: {} ({} per mode)", g.layer_count, g.layers_per_mode);
        for m in 0..g.mode_count() {
            if let Ok(r) = g.mode_layers(m) {
                println!("  {}: layers {}-{}", g.mode_name(m), r.start, r.end - 1);
            }
        }
        println!(
            "matrix: {} rows x {} cols + {} extra entries (knob)",
            g.rows, g.cols, g.extras
        );
        println!(
            "keymap size: {} bytes per layer, {} total",
            g.layer_bytes(),
            g.total_bytes()
        );
        if !kb.layout.has_names() {
            println!("note: no named positions for this matrix; use row,col or #index");
        }
        Ok(())
    })
}

pub fn run_dump(api: &HidApi, args: &KeymapDumpArgs) -> Result<()> {
    with_keyboard(api, &args.device, |kb| {
        let g = kb.geometry();
        let keymap = kb.read(args.defaults)?;
        let factory = if args.changed {
            Some(kb.read(true)?)
        } else {
            None
        };

        if let Some(path) = &args.raw_out {
            std::fs::write(path, &keymap.bytes)
                .with_context(|| format!("failed to write {}", path.display()))?;
            eprintln!("saved {} bytes to {}", keymap.bytes.len(), path.display());
        }

        let layers: Vec<u8> = match (&args.layer, args.mode) {
            (Some(spec), _) => vec![parse_layer_spec(spec, &g)?],
            (None, Some(mode)) => g.mode_layers(mode.index())?.collect(),
            (None, None) => (0..g.layer_count).collect(),
        };

        let layout = &kb.layout;
        let mut json_layers = Vec::new();
        for &layer in &layers {
            let mut keys = Vec::new();
            let mut lines = Vec::new();
            for index in 0..g.keys_per_layer() {
                let code = keymap.get(layer, index);
                let default = factory.as_ref().map(|f| f.get(layer, index));
                if let Some(d) = default
                    && d == code
                {
                    continue;
                }
                if args.json {
                    let mut v = position_json(layout, layer, index);
                    v["code"] = json!(format_hex(code));
                    v["value"] = json!(code);
                    v["name"] = json!(keycode_name(code));
                    if let Some(d) = default {
                        v["factory"] = code_json(d);
                    }
                    keys.push(v);
                    continue;
                }
                if !args.changed && layout.name_of(index).is_none() && code == 0 {
                    continue;
                }
                let rc = match g.row_col(index) {
                    Some((r, c)) => format!("{r},{c}"),
                    None => format!("#{index}"),
                };
                let mut line = format!(
                    "  {:<10} {:<6} {}  {}",
                    layout.label(index),
                    rc,
                    format_hex(code),
                    keycode_name(code).unwrap_or_default()
                );
                if let Some(d) = default {
                    line.push_str(&format!(
                        "  (factory {})",
                        keycode_name(d).unwrap_or_else(|| format_hex(d))
                    ));
                }
                lines.push(line.trim_end().to_string());
            }
            if !args.json && !(args.changed && lines.is_empty()) {
                println!("layer {layer} ({})", g.layer_label(layer));
                for line in &lines {
                    println!("{line}");
                }
            }
            if args.json {
                json_layers.push(json!({
                    "layer": layer,
                    "layer_name": g.layer_label(layer),
                    "keys": keys,
                }));
            }
        }
        if args.json {
            print_json(&json!({
                "source": if args.defaults { "factory" } else { "active" },
                "changed_only": args.changed,
                "layers": json_layers,
            }))?;
        }
        Ok(())
    })
}

pub fn run_export(api: &HidApi, args: &KeymapExportArgs) -> Result<()> {
    with_keyboard(api, &args.device, |kb| {
        let current = kb.read(false)?;
        let factory = kb.read(true)?;
        print!("{}", export_config(&current, &factory, &kb.layout));
        Ok(())
    })
}

pub fn run_set(api: &HidApi, args: &KeymapSetArgs) -> Result<()> {
    let code = parse_keycode(&args.code)?;
    with_keyboard(api, &args.device, |kb| {
        let layer = parse_layer_spec(&args.layer, &kb.geometry())?;
        let index = kb.layout.parse_position(&args.position)?;
        let current = kb.read(false)?;
        let change = Change {
            layer,
            index,
            from: current.get(layer, index),
            to: code,
        };
        let unchanged = change.from == change.to;

        let backup = if args.dry_run {
            None
        } else {
            // An explicit `set` always writes (even an unchanged value) and verifies.
            Some(kb.write_and_verify(&current, std::slice::from_ref(&change))?)
        };

        if args.json {
            let mut v = change_json(&kb.layout, &change);
            v["unchanged"] = json!(unchanged);
            v["dry_run"] = json!(args.dry_run);
            v["verified"] = json!(backup.is_some());
            v["backup"] = json!(backup.map(|p| p.display().to_string()));
            return print_json(&v);
        }
        println!("{}", change.describe(&kb.layout));
        match (args.dry_run, unchanged) {
            (true, _) => println!("dry run: nothing written"),
            (false, true) => println!("value unchanged; rewritten and verified"),
            (false, false) => println!("written and verified"),
        }
        Ok(())
    })
}

pub fn run_apply(api: &HidApi, args: &KeymapApplyArgs) -> Result<()> {
    let text = std::fs::read_to_string(&args.file)
        .with_context(|| format!("failed to read {}", args.file.display()))?;
    with_keyboard(api, &args.device, |kb| {
        let assignments: Vec<Assignment> = parse_config(&text, &kb.layout)
            .with_context(|| format!("invalid keymap file {}", args.file.display()))?;
        let current = kb.read(false)?;
        let changes = diff_assignments(&current, &assignments);

        let backup = if args.dry_run || changes.is_empty() {
            None
        } else {
            Some(kb.write_and_verify(&current, &changes)?)
        };

        if args.json {
            let list: Vec<Value> = changes.iter().map(|c| change_json(&kb.layout, c)).collect();
            return print_json(&json!({
                "file": args.file.display().to_string(),
                "keys_in_file": assignments.len(),
                "changes": list,
                "dry_run": args.dry_run,
                "written": backup.is_some(),
                "verified": backup.is_some(),
                "backup": backup.map(|p| p.display().to_string()),
            }));
        }
        if changes.is_empty() {
            println!(
                "no changes ({} keys in file already match)",
                assignments.len()
            );
            return Ok(());
        }
        print_changes(&kb.layout, &changes);
        if args.dry_run {
            println!("dry run: {} change(s), nothing written", changes.len());
        } else {
            println!("{} change(s) written and verified", changes.len());
        }
        Ok(())
    })
}

pub fn run_reset(api: &HidApi, args: &KeymapResetArgs) -> Result<()> {
    with_keyboard(api, &args.device, |kb| {
        let g = kb.geometry();
        let layers = match args.mode {
            KeymapResetMode::Mac => g.mode_layers(0)?,
            KeymapResetMode::Windows => g.mode_layers(1)?,
            KeymapResetMode::All => 0..g.layer_count,
        };
        if !args.yes {
            bail!(
                "refusing to reset layers {}-{} to factory without --yes",
                layers.start,
                layers.end - 1
            );
        }
        let current = kb.read(false)?;
        let factory = kb.read(true)?;
        let backup = backup_keymap(&current)?;
        eprintln!("backup: {}", backup.display());

        let mut desired = current.clone();
        for layer in layers.clone() {
            for index in 0..g.keys_per_layer() {
                desired.set(layer, index, factory.get(layer, index));
            }
        }
        let changes = diff_keymaps(&current, &desired);
        kb.protocol
            .restore_layers(layers.start, layers.end - layers.start)
            .with_context(|| {
                format!(
                    "restore with: nuphyctl keymap restore-backup {}",
                    backup.display()
                )
            })?;
        kb.verify(&desired, &backup)?;

        if args.json {
            let list: Vec<Value> = changes.iter().map(|c| change_json(&kb.layout, c)).collect();
            return print_json(&json!({
                "layers": layers.collect::<Vec<u8>>(),
                "changes": list,
                "verified": true,
                "backup": backup.display().to_string(),
            }));
        }
        print_changes(&kb.layout, &changes);
        println!(
            "layers {}-{} reset to factory ({} key(s) changed), verified",
            layers.start,
            layers.end - 1,
            changes.len()
        );
        Ok(())
    })
}

pub fn run_restore_backup(api: &HidApi, args: &KeymapRestoreBackupArgs) -> Result<()> {
    let bytes = std::fs::read(&args.file)
        .with_context(|| format!("failed to read {}", args.file.display()))?;
    with_keyboard(api, &args.device, |kb| {
        let desired = Keymap::from_bytes(kb.geometry(), bytes)
            .with_context(|| format!("{} does not match this keyboard", args.file.display()))?;
        let current = kb.read(false)?;
        let changes = diff_keymaps(&current, &desired);

        let backup = if args.dry_run || changes.is_empty() {
            None
        } else {
            Some(kb.write_and_verify(&current, &changes)?)
        };

        if args.json {
            let list: Vec<Value> = changes.iter().map(|c| change_json(&kb.layout, c)).collect();
            return print_json(&json!({
                "file": args.file.display().to_string(),
                "changes": list,
                "dry_run": args.dry_run,
                "written": backup.is_some(),
                "verified": backup.is_some(),
                "backup": backup.map(|p| p.display().to_string()),
            }));
        }
        if changes.is_empty() {
            println!(
                "no changes (keyboard already matches {})",
                args.file.display()
            );
            return Ok(());
        }
        print_changes(&kb.layout, &changes);
        if args.dry_run {
            println!("dry run: {} change(s), nothing written", changes.len());
        } else {
            println!("{} change(s) written and verified", changes.len());
        }
        Ok(())
    })
}

pub fn run_keycodes(args: &KeymapListArgs) -> Result<()> {
    if args.json {
        let list: Vec<Value> = KEYCODES
            .iter()
            .map(|k| {
                json!({
                    "name": k.name,
                    "code": format_hex(k.code),
                    "value": k.code,
                    "aliases": k.aliases,
                    "group": k.group.name(),
                    "description": k.description,
                })
            })
            .collect();
        return print_json(&json!({
            "keycodes": list,
            "patterns": [
                { "pattern": "MO(n)", "code": "0x5220 | n", "description": "momentary layer n while held (0-31)" },
                { "pattern": "0xNNNN", "description": "any raw 16-bit keycode" },
            ],
        }));
    }
    for k in KEYCODES {
        let aliases = if k.aliases.is_empty() {
            String::new()
        } else {
            format!(" [{}]", k.aliases.join(", "))
        };
        println!(
            "{}  {:<26} {:<8} {}{}",
            format_hex(k.code),
            k.name,
            k.group.name(),
            k.description,
            aliases
        );
    }
    println!("0x522n  MO(n)                      layer    momentary layer n while held");
    println!("any other value can be given as hex, e.g. 0x7e0a");
    Ok(())
}

pub fn run_positions(args: &KeymapListArgs) -> Result<()> {
    let g = Geometry::AIR75_V3;
    if args.json {
        let list: Vec<Value> = AIR75_V3_POSITIONS
            .iter()
            .map(|p| {
                let (row, col) = match g.row_col(p.index) {
                    Some((r, c)) => (json!(r), json!(c)),
                    None => (Value::Null, Value::Null),
                };
                json!({
                    "name": p.name,
                    "index": p.index,
                    "row": row,
                    "col": col,
                    "aliases": p.aliases,
                    "description": p.description,
                })
            })
            .collect();
        return print_json(&json!({ "layout": "Air75 V3", "positions": list }));
    }
    println!("Air75 V3 positions (Windows legends; also accepted: row,col and #index)");
    for p in AIR75_V3_POSITIONS {
        let rc = match g.row_col(p.index) {
            Some((r, c)) => format!("{r},{c}"),
            None => "-".to_string(),
        };
        let mut line = format!("  {:<9} {:<6} #{:<3}", p.name, rc, p.index);
        if !p.aliases.is_empty() {
            line.push_str(&format!(" [{}]", p.aliases.join(", ")));
        }
        if !p.description.is_empty() {
            line.push_str(&format!(" {}", p.description));
        }
        println!("{}", line.trim_end());
    }
    Ok(())
}
