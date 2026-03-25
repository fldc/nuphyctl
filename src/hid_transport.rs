use crate::cli::DeviceSelector;
use anyhow::{anyhow, bail, Context, Result};
use hidapi::{DeviceInfo, HidApi, HidDevice};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

pub const REPORT_LEN: usize = 64;
pub type Report = [u8; REPORT_LEN];

#[derive(Clone, Copy, Debug)]
pub struct HidResponder {
    timeout: Duration,
}

impl HidResponder {
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }

    pub fn send_and_expect_ack(
        &self,
        dev: &HidDevice,
        report_id: u8,
        packet: &Report,
        expected_cmd: u8,
    ) -> Result<Report> {
        send_report(dev, report_id, packet)?;
        wait_for_response(dev, expected_cmd, self.timeout)
    }
}

pub fn list_devices(api: &HidApi) {
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
}

pub fn open_selected_device(api: &HidApi, selector: &DeviceSelector) -> Result<HidDevice> {
    let selected: Vec<&DeviceInfo> = api
        .device_list()
        .filter(|d| {
            let vid_pid_match = match (selector.vid, selector.pid) {
                (Some(vid), Some(pid)) => d.vendor_id() == vid && d.product_id() == pid,
                (Some(vid), None) => d.vendor_id() == vid,
                (None, Some(pid)) => d.product_id() == pid,
                (None, None) => true,
            };
            let path_match = selector
                .path
                .as_ref()
                .map(|p| d.path().to_string_lossy() == p.as_str())
                .unwrap_or(true);
            let iface_match = selector
                .iface
                .map(|iface| d.interface_number() == iface)
                .unwrap_or(true);
            let usage_page_match = selector
                .usage_page
                .map(|usage_page| d.usage_page() == usage_page)
                .unwrap_or(true);
            let usage_match = selector
                .usage
                .map(|usage| d.usage() == usage)
                .unwrap_or(true);
            vid_pid_match && path_match && iface_match && usage_page_match && usage_match
        })
        .collect();

    if selected.is_empty() {
        bail!("no matching HID device found; try `nuphyctl list`");
    }

    if selector.path.is_none()
        && selector.iface.is_none()
        && selector.usage_page.is_none()
        && selector.usage.is_none()
    {
        let mut ranked: Vec<(&DeviceInfo, i32)> = selected
            .iter()
            .copied()
            .map(|d| {
                let mut score = 0;
                if d.usage_page() == 0x0001 && d.usage() == 0x0000 {
                    score += 100;
                }
                if d.interface_number() == 3 {
                    score += 50;
                }
                if d.usage_page() == 0x0001 && d.usage() == 0x0080 {
                    score -= 20;
                }
                if d.usage_page() == 0x0001 && d.usage() == 0x0006 {
                    score -= 30;
                }
                (d, score)
            })
            .collect();

        ranked.sort_by(|a, b| b.1.cmp(&a.1));

        if let Some((best, best_score)) = ranked.first().copied() {
            let next_score = ranked.get(1).map(|(_, s)| *s).unwrap_or(i32::MIN);
            if best_score > 0 && best_score > next_score {
                return api
                    .open_path(best.path())
                    .with_context(|| format!("open failed for path {:?}", best.path()));
            }
        }
    }

    let unique_paths: BTreeSet<String> = selected
        .iter()
        .map(|d| d.path().to_string_lossy().into_owned())
        .collect();

    if unique_paths.len() == 1 {
        let only = selected[0];
        return api
            .open_path(only.path())
            .with_context(|| format!("open failed for path {:?}", only.path()));
    }

    match selected.len() {
        1 => {
            let d = selected[0];
            api.open_path(d.path())
                .with_context(|| format!("open failed for path {:?}", d.path()))
        }
        _ => {
            let mut lines = String::new();
            for d in selected {
                lines.push_str(&format!(
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
            bail!(
                "multiple matching HID devices; specify --path or narrow with --iface/--usage-page/--usage. candidates:\n{}",
                lines
            )
        }
    }
}

pub fn send_report(dev: &HidDevice, report_id: u8, data: &[u8]) -> Result<()> {
    let mut with_rid = Vec::with_capacity(data.len() + 1);
    with_rid.push(report_id);
    with_rid.extend_from_slice(data);

    let mut errors = Vec::new();

    match dev.send_output_report(&with_rid) {
        Ok(()) => return Ok(()),
        Err(err) => errors.push(format!("send_output_report(with_rid) failed: {err}")),
    }

    match dev.write(&with_rid) {
        Ok(written) if written == with_rid.len() => return Ok(()),
        Ok(written) => errors.push(format!(
            "write(with_rid) short write: wrote {} bytes, expected {}",
            written,
            with_rid.len()
        )),
        Err(err) => errors.push(format!("write(with_rid) failed: {err}")),
    }

    if report_id == 0 {
        match dev.write(data) {
            Ok(written) if written == data.len() => return Ok(()),
            Ok(written) => errors.push(format!(
                "write(no_rid) short write: wrote {} bytes, expected {}",
                written,
                data.len()
            )),
            Err(err) => errors.push(format!("write(no_rid) failed: {err}")),
        }
    }

    Err(anyhow!(errors.join("; ")))
}

pub fn read_input_report(dev: &HidDevice, timeout: Duration) -> Result<Option<Report>> {
    let timeout_ms = timeout.as_millis().clamp(1, i32::MAX as u128) as i32;
    let mut buf = [0u8; REPORT_LEN + 1];
    let n = dev
        .read_timeout(&mut buf, timeout_ms)
        .with_context(|| format!("hid read_timeout failed ({} ms)", timeout_ms))?;

    if n == 0 {
        return Ok(None);
    }

    if n == REPORT_LEN {
        let mut out = [0u8; REPORT_LEN];
        out.copy_from_slice(&buf[..REPORT_LEN]);
        return Ok(Some(out));
    }

    if n == REPORT_LEN + 1 {
        let mut out = [0u8; REPORT_LEN];
        out.copy_from_slice(&buf[1..]);
        return Ok(Some(out));
    }

    bail!("unexpected HID input report length: {} bytes", n);
}

pub fn clear_input_reports(dev: &HidDevice) -> Result<()> {
    let mut buf = [0u8; REPORT_LEN + 1];

    loop {
        let n = dev
            .read_timeout(&mut buf, 0)
            .context("failed to drain stale HID input reports")?;
        if n == 0 {
            return Ok(());
        }
    }
}

fn wait_for_response(dev: &HidDevice, expected_cmd: u8, timeout: Duration) -> Result<Report> {
    let started = Instant::now();

    loop {
        let elapsed = started.elapsed();
        if elapsed >= timeout {
            bail!(
                "timeout waiting for HID response command 0x{:02x}",
                expected_cmd
            );
        }

        let remaining = timeout - elapsed;
        let step = remaining.min(Duration::from_millis(80));
        if let Some(report) = read_input_report(dev, step)? {
            if report[1] == expected_cmd {
                return Ok(report);
            }
        }
    }
}
