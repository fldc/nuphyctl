# nuphyctl

CLI for sending NuPhy keyboard HID commands.

## Build

```bash
cargo build
```

## Usage

List HID devices:

```bash
cargo run -- list
```

Set static color (`#RRGGBB`):

```bash
cargo run -- rgb set --hex ff0000
```

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
