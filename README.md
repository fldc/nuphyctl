# nuphyctl

`nuphyctl` is a Rust CLI for sending NuPhy keyboard HID commands to boards supported by [drive.nuphy.io](https://drive.nuphy.io).

It currently supports:

- listing visible HID devices
- printing supported command paths
- setting main/backlight RGB effects
- setting side-light effects
- setting decorative/strip-light effects
- reading, remapping, backing up and restoring the keymap (declaratively from a TOML file)
- sending raw 64-byte HID reports for reverse engineering

## Build

```bash
cargo build
```

Run directly from the repo during development:

```bash
cargo run -- --help
```

Or install it locally:

```bash
cargo install --path .
```

## Commands

List HID devices visible to `hidapi`:

```bash
cargo run -- list
```

Print all supported command paths:

```bash
cargo run -- commands
```

Show help for a subcommand:

```bash
cargo run -- rgb set --help
```

## Lighting examples

Set a static backlight color (`#RRGGBB` or `RRGGBB`):

```bash
cargo run -- rgb set --hex ff0000
```

Set backlight brightness (`0-100`):

```bash
cargo run -- rgb set --hex ff0000 --brightness 35
```

Set an animated backlight effect with speed and direction:

```bash
cargo run -- rgb set --effect wave --hex 00aaff --speed 3 --direction left
```

Use a preset palette instead of a custom RGB color:

```bash
cargo run -- rgb set --effect wave --hex 000000 --color-mode preset --palette-index 2
```

Set a side-light effect with a custom color:

```bash
cargo run -- rgb side --effect static --hex ffffff --brightness 70 --speed 2
```

For side-light effects that do not use custom color, `--hex` is optional:

```bash
cargo run -- rgb side --effect neon --brightness 70 --speed 2
```

Set a decorative/strip-light effect (experimental, model-dependent offsets):

```bash
cargo run -- rgb decorative --effect static --hex ff8800 --base-offset 17
```

If your model uses a different decorative channel offset, override `--base-offset`.
Observed values so far include `17` for some 2-channel layouts and `35` for some 3-channel layouts.

## Effects

Main/backlight effects:

`ray`, `stair`, `static`, `breath`, `flower`, `wave`, `ripple`, `spout`, `galaxy`, `rotation`, `ripple2`, `point`, `grid`, `time`, `rain`, `ribbon`, `gaming`, `identify`, `windmill`, `diagonal`

Side/decorative effects:

`time`, `neon`, `static`, `breathe`, `rhythm`

## Keymap

The `keymap` commands read and write the keymap stored on the keyboard (the same data
NuPhy IO edits). Every command that changes the keyboard first saves a binary backup of
the whole keymap, writes only keys whose value differs, then reads the keymap back and
fails unless it matches exactly. Changes persist across replugs; there is no separate
save step.

Named positions follow the Air75 V3 layout (Windows legends). Any board also accepts
`row,col` and `#index` positions.

Show firmware version, current mode and keymap geometry:

```bash
nuphyctl keymap info
nuphyctl keymap info --json
```

Layers: `0-3` are Mac mode, `4-7` Windows mode. Within a mode the layers are `base`,
`fn`, `fn2` and `fn3`, so `windows.fn` is layer `5`. Commands accept either form.

Print the keymap, a single layer, or one mode (`--defaults` reads the factory keymap,
`--changed` shows only keys that differ from it):

```bash
nuphyctl keymap dump --layer windows.base
nuphyctl keymap dump --mode windows --json
nuphyctl keymap dump --changed
```

Save a full binary backup (all layers) and write one back later. Only differing keys are
written, and the result is verified:

```bash
nuphyctl keymap dump --raw-out air75.bin
nuphyctl keymap restore-backup air75.bin --dry-run
nuphyctl keymap restore-backup air75.bin
```

Set one key. Positions are names (`F1`, `ESC`, `YELLOW`, `KNOB`, `KNOB_CCW`, ...), `row,col`
or `#index`; keycodes are names (`BRID`, `F20`, `MISSION_CONTROL`, `DND`, `MO(5)`, `TRNS`, ...)
or hex (`0x00be`):

```bash
nuphyctl keymap set --layer windows.base F1 BRID
nuphyctl keymap set --layer 4 3,12 MUTE
nuphyctl keymap set --layer 4 KNOB_CW 0x00a9 --json
```

List the names:

```bash
nuphyctl keymap keycodes
nuphyctl keymap positions --json
```

### Declarative keymap files

Keep the keymap in a TOML file (for example in your dotfiles) and apply it. Only keys
listed in the file are touched. `apply` prints the diff (current -> desired), writes only
the changes, verifies them, and is idempotent: a second run reports `no changes`.

```toml
[windows.base]          # layer 4
F1 = "BRID"
F2 = "BRIU"
F3 = "MISSION_CONTROL"
F6 = "DND"
YELLOW = "DEL"
"3,12" = "MUTE"         # row,col and "#index" work too

[windows.base.knob]     # same as KNOB_CCW / KNOB_CW / KNOB
ccw = "VOLD"
cw = "VOLU"

[windows.fn]            # layer 5
F1 = "F1"
F2 = 0x3b               # integers are raw keycodes

[layers.6]              # absolute layer number
ESC = "TRNS"
```

```bash
nuphyctl keymap apply keymap.toml --dry-run
nuphyctl keymap apply keymap.toml
nuphyctl keymap apply keymap.toml --json
```

Sections are `[mac.<layer>]`, `[windows.<layer>]` (`base`, `fn`, `fn2`, `fn3`) and
`[layers.<n>]`. Unknown sections, positions or keycodes, and two entries for the same key,
are errors; nothing is written when the file does not parse.

To start a file from the keyboard's current state, export the keys that differ from the
factory keymap:

```bash
nuphyctl keymap export > keymap.toml
```

### Backups and reset

Before any write, `set`, `apply`, `restore-backup` and `reset` save the current keymap to
`$XDG_STATE_HOME/nuphyctl/keymap-<UTC timestamp>.bin` (default
`~/.local/state/nuphyctl/`) and print the path. Restore one with
`nuphyctl keymap restore-backup <file>`.

Reset a mode's layers (or all layers) to the factory keymap. `--yes` is required:

```bash
nuphyctl keymap reset --mode windows --yes
```

## Device selection

If more than one matching HID interface is present, narrow the target with one or more selector flags.

Target a specific vendor/product pair:

```bash
cargo run -- rgb set --hex 00ff00 --vid 0x19f5 --pid 0x1028
```

Target a specific `hidraw` path:

```bash
cargo run -- rgb set --hex 00ff00 --path /dev/hidraw5
```

Narrow by interface and usage fields from `list` output:

```bash
cargo run -- rgb set --hex 00ff00 --vid 0x19f5 --pid 0x1028 --iface 3 --usage-page 0x0001 --usage 0x0000
```

Available selectors:

- `--vid`
- `--pid`
- `--path`
- `--iface`
- `--usage-page`
- `--usage`

If you do not pass a selector, `nuphyctl` tries to choose the most likely keyboard control interface automatically.

## Raw reports

Send a raw 64-byte HID packet:

```bash
cargo run -- raw send --hex "55d60093656564650000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
```

You can also override the HID report ID when needed:

```bash
cargo run -- raw send --report-id 0 --hex "...128 hex chars..."
```

## Notes

- `rgb set`, `rgb side`, and `rgb decorative` automatically retry a few common transient HID failures
- `rgb side` and `rgb decorative` require `--hex` only for effects that use custom RGB color
- `raw send` expects exactly 64 payload bytes
- `keymap` commands negotiate a new session key, so a NuPhy IO tab that is open at the same time may need a reload afterwards

## Reverse engineering

Detailed protocol notes and the DevTools capture workflow live in [docs/reverse-engineering.md](docs/reverse-engineering.md).
