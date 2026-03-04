use anyhow::{bail, Context, Result};
use hidapi::{DeviceInfo, HidApi, HidDevice};

use crate::cli::DeviceSelector;

const DEFAULT_NUPHY_VID: u16 = 0x19f5;
const DEFAULT_AIR75_V3_PID: u16 = 0x1028;

pub fn list_devices(api: &HidApi) -> Result<()> {
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

pub fn open_selected_device(api: &HidApi, selector: &DeviceSelector) -> Result<HidDevice> {
    let selected: Vec<&DeviceInfo> = api
        .device_list()
        .filter(|d| matches_selector(d, selector))
        .collect();
    if selected.is_empty() {
        bail!("no matching HID device found; try `nuphyctl list`");
    }

    if !selector.has_routing_overrides() {
        if let Some(d) = auto_pick_device(&selected) {
            return open_path(api, d);
        }
    }

    if selected.len() == 1 {
        return open_path(api, selected[0]);
    }

    if all_same_path(&selected) {
        return open_path(api, selected[0]);
    }

    bail!(
        "multiple matching HID devices; specify --path or narrow with --iface/--usage-page/--usage. candidates:\n{}",
        format_candidates(&selected)
    )
}

pub fn send_report(dev: &HidDevice, report_id: u8, data: &[u8]) -> Result<()> {
    if data.len() > 64 {
        bail!("report payload too large: {} bytes", data.len());
    }

    let mut report = [0u8; 65];
    report[0] = report_id;
    let end = data.len() + 1;
    report[1..end].copy_from_slice(data);
    dev.send_output_report(&report[..end])
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}

fn matches_selector(device: &DeviceInfo, selector: &DeviceSelector) -> bool {
    let apply_default_vid_pid =
        selector.vid.is_none() && selector.pid.is_none() && selector.path.is_none();
    let (effective_vid, effective_pid) = if apply_default_vid_pid {
        (Some(DEFAULT_NUPHY_VID), Some(DEFAULT_AIR75_V3_PID))
    } else {
        (selector.vid, selector.pid)
    };

    let vid_pid_match = match (effective_vid, effective_pid) {
        (Some(vid), Some(pid)) => device.vendor_id() == vid && device.product_id() == pid,
        (Some(vid), None) => device.vendor_id() == vid,
        (None, Some(pid)) => device.product_id() == pid,
        (None, None) => true,
    };

    let path_match = selector
        .path
        .as_ref()
        .map(|p| device.path().to_string_lossy().as_ref() == p.as_str())
        .unwrap_or(true);
    let iface_match = selector
        .iface
        .map(|iface| device.interface_number() == iface)
        .unwrap_or(true);
    let usage_page_match = selector
        .usage_page
        .map(|usage_page| device.usage_page() == usage_page)
        .unwrap_or(true);
    let usage_match = selector
        .usage
        .map(|usage| device.usage() == usage)
        .unwrap_or(true);

    vid_pid_match && path_match && iface_match && usage_page_match && usage_match
}

fn open_path(api: &HidApi, device: &DeviceInfo) -> Result<HidDevice> {
    api.open_path(device.path())
        .with_context(|| format!("open failed for path {:?}", device.path()))
}

fn auto_pick_device<'a>(devices: &'a [&'a DeviceInfo]) -> Option<&'a DeviceInfo> {
    // Heuristic based on observed Air75 V3 layout.
    let mut ranked: Vec<(&DeviceInfo, i32)> = devices
        .iter()
        .copied()
        .map(|d| (d, device_score(d)))
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1));

    let (best, best_score) = ranked.first().copied()?;
    let next_score = ranked.get(1).map(|(_, score)| *score).unwrap_or(i32::MIN);

    if best_score > 0 && best_score > next_score {
        Some(best)
    } else {
        None
    }
}

fn device_score(device: &DeviceInfo) -> i32 {
    let mut score = 0;
    if device.usage_page() == 0x0001 && device.usage() == 0x0000 {
        score += 100;
    }
    if device.usage_page() == 0x0001 && device.usage() == 0x0080 {
        score -= 20;
    }
    if device.usage_page() == 0x0001 && device.usage() == 0x0006 {
        score -= 30;
    }
    score
}

fn all_same_path(devices: &[&DeviceInfo]) -> bool {
    let Some(first) = devices.first() else {
        return true;
    };
    devices.iter().all(|d| d.path() == first.path())
}

fn format_candidates(devices: &[&DeviceInfo]) -> String {
    let mut out = String::new();
    for d in devices {
        out.push_str(&format!(
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
    out
}
