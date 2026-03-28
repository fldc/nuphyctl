# nuphyctl

CLI for sending NuPhy keyboard HID commands (for keyboards supported by [drive.nuphy.io](https://drive.nuphy.io)).

## Build

```bash
cargo build
```

## Usage

List HID devices:

```bash
cargo run -- list
```

List all available command paths:

```bash
cargo run -- commands
```

Set RGB effect and color (`#RRGGBB`):

```bash
cargo run -- rgb set --hex ff0000
```

Backlight supports speed, direction, and color source:

```bash
cargo run -- rgb set --effect wave --hex 00aaff --speed 3 --direction left --color-mode custom
```

Select a specific lighting effect from NuPhy Drive:

```bash
cargo run -- rgb set --effect wave --hex 00aaff
```

Available effects: `ray`, `stair`, `static`, `breath`, `flower`, `wave`, `ripple`, `spout`, `galaxy`, `rotation`, `ripple2`, `point`, `grid`, `time`, `rain`, `ribbon`, `gaming`, `identify`, `windmill`, `diagonal`.

Set side-light effect:

```bash
cargo run -- rgb side --effect neon --hex ffffff --brightness 70 --speed 2
```

Side-light effects: `time`, `neon`, `static`, `breathe`, `rhythm`.

Set decorative/strip light effect (experimental, model-dependent offsets):

```bash
cargo run -- rgb decorative --effect static --hex ff8800 --base-offset 17
```

If your model uses a different decorative channel offset, override `--base-offset` (common values observed: `17` for some 2-channel layouts, `35` for 3-channel layouts).

Set static color with brightness (`0-100`):

```bash
cargo run -- rgb set --hex ff0000 --brightness 35
```

If multiple HID devices are present, target one explicitly:

```bash
cargo run -- rgb set --hex 00ff00 --vid 0x19f5 --pid 0x1028
```

When no `--vid/--pid` is provided, `nuphyctl` defaults to Air75 V3 (`0x19f5:0x1028`).

For composite devices with several interfaces (common on keyboards), select by hidraw path:

```bash
cargo run -- rgb set --hex 00ff00 --path /dev/hidraw5
```

You can also narrow by interface or usage fields from `list` output:

```bash
cargo run -- rgb set --hex 00ff00 --vid 0x19f5 --pid 0x1028 --iface 3 --usage-page 0x0001 --usage 0x0000
```

Send a raw 64-byte packet:

```bash
cargo run -- raw send --hex "55d60093656564650000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
```

## Reverse engineering

Detailed protocol notes and DevTools workflow are in:

- [docs/reverse-engineering.md](docs/reverse-engineering.md)
