//! Sleep settings use the NuPhy IO mechanical protocol, not the HE config block.
use crate::cli::{SettingsGetArgs, SettingsSetArgs};
use crate::hid_transport::{clear_input_reports, open_selected_device};
use crate::nuphy_protocol::KeyboardProtocol;
use anyhow::{Context, Result, bail};
use hidapi::HidApi;
use serde_json::{Value, json};

pub fn run_get(api: &HidApi, args: &SettingsGetArgs) -> Result<()> {
    let dev = open_selected_device(api, &args.device)?;
    clear_input_reports(&dev)?;
    let protocol = KeyboardProtocol::new(&dev)?;
    let sleep = protocol.read_sleep_settings()?;
    let accessory = args
        .accessory
        .then(|| protocol.read_accessory())
        .transpose()?;
    print_settings(Some(sleep), accessory, args.json, None);
    Ok(())
}

pub fn run_set(api: &HidApi, args: &SettingsSetArgs) -> Result<()> {
    let dev = open_selected_device(api, &args.device)?;
    clear_input_reports(&dev)?;
    let protocol = KeyboardProtocol::new(&dev)?;
    let changes_sleep =
        args.auto_sleep.is_some() || args.level1_minutes.is_some() || args.level2_minutes.is_some();
    // Read everything needed before performing any writes.
    let before_sleep = changes_sleep
        .then(|| protocol.read_sleep_settings())
        .transpose()?;
    let desired_sleep = before_sleep.map(|current| update_sleep(current, args));
    let before_accessory = args
        .accessory
        .map(|_| protocol.read_accessory())
        .transpose()?;
    let desired_accessory = args.accessory.map(|value| value.protocol_value());

    if !args.dry_run {
        if let Some(desired) = desired_sleep {
            if Some(desired) != before_sleep {
                protocol.write_sleep_settings(&desired)?;
            }
            let actual = protocol
                .read_sleep_settings()
                .context("sleep settings may have been written, but read-back failed")?;
            if actual != desired {
                bail!("sleep settings verification failed: expected {desired:?}, got {actual:?}");
            }
        }
        if let Some(desired) = desired_accessory {
            if Some(desired) != before_accessory {
                protocol.write_accessory(desired)?;
            }
            let actual = protocol
                .read_accessory()
                .context("accessory setting may have been written, but read-back failed")?;
            if actual != desired {
                bail!("accessory verification failed: expected {desired}, got {actual}");
            }
        }
    }
    print_settings(
        desired_sleep,
        desired_accessory,
        args.json,
        Some(args.dry_run),
    );
    Ok(())
}

fn update_sleep(mut current: [u8; 4], args: &SettingsSetArgs) -> [u8; 4] {
    if let Some(enabled) = args.auto_sleep {
        current[0] = u8::from(enabled);
    }
    if let Some(minutes) = args.level1_minutes {
        current[1] = minutes;
    }
    if let Some(minutes) = args.level2_minutes {
        current[2] = minutes;
    }
    current
}

fn print_settings(
    sleep: Option<[u8; 4]>,
    accessory: Option<u8>,
    json_output: bool,
    dry_run: Option<bool>,
) {
    let mut output = json!({});
    if let Some(data) = sleep {
        output["auto_sleep"] = json!(data[0] != 0);
        output["level1_minutes"] = json!(data[1]);
        output["level2_minutes"] = json!(data[2]);
    }
    if let Some(value) = accessory {
        output["accessory"] = match value {
            0 => json!("knob"),
            1 => json!("button"),
            _ => json!(format!("unknown ({value})")),
        };
    }
    if let Some(dry_run) = dry_run {
        output["dry_run"] = json!(dry_run);
    }
    if json_output {
        println!("{output}");
    } else {
        if dry_run == Some(true) {
            println!("Proposed settings (no writes):");
        }
        for (key, value) in output.as_object().expect("settings object") {
            if key == "dry_run" {
                continue;
            }
            let text = match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            println!("{key}: {text}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Command, SettingsSubcommand};
    use clap::Parser;

    fn set_args(flags: &[&str]) -> SettingsSetArgs {
        let mut argv = vec!["nuphyctl", "settings", "set"];
        argv.extend_from_slice(flags);
        let Command::Settings(command) = Cli::try_parse_from(argv).unwrap().command else {
            panic!()
        };
        let SettingsSubcommand::Set(args) = command.action else {
            panic!()
        };
        args
    }

    #[test]
    fn updates_preserve_unrequested_fields_and_reserved_byte() {
        let current = [1, 6, 24, 0xa5];
        assert_eq!(
            update_sleep(current, &set_args(&["--level1-minutes", "10"])),
            [1, 10, 24, 0xa5]
        );
        assert_eq!(
            update_sleep(current, &set_args(&["--auto-sleep", "off"])),
            [0, 6, 24, 0xa5]
        );
        assert_eq!(
            update_sleep(
                current,
                &set_args(&["--level2-minutes", "127", "--auto-sleep", "on"])
            ),
            [1, 6, 127, 0xa5]
        );
    }

    #[test]
    fn rejects_invalid_or_empty_updates() {
        for flags in [
            vec![],
            vec!["--json"],
            vec!["--dry-run"],
            vec!["--level1-minutes", "4"],
            vec!["--level2-minutes", "128"],
            vec!["--auto-sleep", "maybe"],
            vec!["--accessory", "slider"],
        ] {
            let mut argv = vec!["nuphyctl", "settings", "set"];
            argv.extend(flags);
            assert!(Cli::try_parse_from(argv).is_err());
        }
        for minutes in ["5", "127"] {
            set_args(&["--level1-minutes", minutes, "--level2-minutes", minutes]);
        }
        assert_eq!(
            set_args(&["--accessory", "knob"])
                .accessory
                .unwrap()
                .protocol_value(),
            0
        );
        assert_eq!(
            set_args(&["--accessory", "button"])
                .accessory
                .unwrap()
                .protocol_value(),
            1
        );
    }
}
