# nuphyctl

`nuphyctl` is a Rust CLI for sending NuPhy keyboard HID commands to boards supported by [drive.nuphy.io](https://drive.nuphy.io).

It currently supports:

- listing visible HID devices
- printing supported command paths
- setting main/backlight RGB effects
- setting side-light effects
- setting decorative/strip-light effects
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

## Reverse engineering

Detailed protocol notes and the DevTools capture workflow live in `docs/reverse-engineering.md`.
