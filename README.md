# nuphyctl

`nuphyctl` is a Rust CLI for sending NuPhy keyboard HID commands to boards supported by [drive.nuphy.io](https://drive.nuphy.io).

- Set main/backlight, side-light, and decorative RGB effects.
- Read, remap, back up, and restore keymaps, including declarative TOML configuration.
- List HID devices and send raw reports for reverse engineering.

## Installation

On Arch Linux, install [`nuphyctl-bin` from the AUR](https://aur.archlinux.org/packages/nuphyctl-bin):

```bash
yay -S nuphyctl-bin
```

For release binaries, source builds, and Cargo installation, see the
[setup guide](docs/setup.md).

### Linux device permissions

The Arch package installs the required udev rule automatically. For other
installation methods, follow the [device permissions setup](docs/setup.md#linux-device-permissions)
before accessing the keyboard. Running `nuphyctl` as root is not required.

## Quick start

```bash
nuphyctl list
nuphyctl rgb set --hex ff8800 --brightness 35
nuphyctl keymap info
nuphyctl keymap dump --changed
```

Use `nuphyctl --help` or `nuphyctl <command> --help` to explore the CLI.

## Documentation

- [Setup](docs/setup.md) — installation, building, and Linux device permissions.
- [Command guide](docs/usage.md) — lighting effects, keymaps, backups, device selection, and raw reports.
- [Reverse engineering](docs/reverse-engineering.md) — protocol notes and the DevTools capture workflow.
