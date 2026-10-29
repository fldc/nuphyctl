# Setup

[Back to README](../README.md) · [Command guide](usage.md)

## Arch Linux

Install [`nuphyctl-bin` from the AUR](https://aur.archlinux.org/packages/nuphyctl-bin):

```bash
yay -S nuphyctl-bin
# or
paru -S nuphyctl-bin
```

The package installs the binary and udev rule. Arch's standard pacman hook reloads
the rules and retriggers devices, including keyboards already plugged in.

## Release binaries

Download the archive for your architecture (`linux-x86_64` or `linux-aarch64`)
from [GitHub Releases](https://github.com/fldc/nuphyctl/releases), then extract it.
From the extracted directory:

```bash
sudo install -Dm755 nuphyctl /usr/local/bin/nuphyctl
sudo install -Dm644 70-nuphyctl.rules /etc/udev/rules.d/70-nuphyctl.rules
```

Then [reload the rules](#reload-device-permissions).

## Build from source

Install Rust/Cargo, a C compiler, pkg-config, and the libudev development files
(provided by `systemd` on Arch or `libudev-dev` on Debian/Ubuntu).

```bash
cargo build --locked
cargo run -- --help
```

To install the binary locally from the checkout:

```bash
cargo install --locked --path .
```

Source builds and `cargo install` do **not** install udev rules. From the checkout:

```bash
sudo install -Dm644 udev/70-nuphyctl.rules /etc/udev/rules.d/70-nuphyctl.rules
```

Then [reload the rules](#reload-device-permissions).

## Linux device permissions

`nuphyctl` needs read/write access to the keyboard's `/dev/hidraw*` devices.
The bundled rule grants access to the **active local seat user** through
systemd-logind, without running `nuphyctl` as root or making devices world-writable.
It matches NuPhy USB devices (vendor ID `19f5`).

Distribution packages should install the rule under `/usr/lib/udev/rules.d/`;
manual installations should use `/etc/udev/rules.d/` as shown above. Avoid installing
duplicate copies in both locations.

### Reload device permissions

After manually installing the rule, or if automatic device retriggering is disabled:

```bash
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=hidraw --property-match=ID_VENDOR_ID=19f5 --action=change
sudo udevadm settle
```

Alternatively, replug the keyboard once after reloading the rules.

Once installed, the rule also covers keyboards connected before login; no per-login
trigger or replug is needed. The filename must sort before `73-seat-late.rules`,
which applies the access ACL. SSH-only and other sessions without an active local
seat are not granted access.

Verify access without sudo:

```bash
nuphyctl keymap info
```
