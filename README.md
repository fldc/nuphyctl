# nuphyctl

CLI for sending NuPhy HID RGB commands.

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

## DevTools reverse-engineering guide

Use this process whenever you want to add new controls (brightness, speed, mode, per-key settings).

### 1) Capture packets in Chrome

Open NuPhy configurator in Chrome, then open DevTools Console and paste:

```js
(() => {
  const toHex = (arr) => Array.from(arr, b => b.toString(16).padStart(2, "0")).join(" ");
  const state = { sessions: [], cur: null, orig: null, installed: false };

  function install() {
    if (state.installed) return;
    const p = HIDDevice.prototype;
    state.orig = {
      sendReport: p.sendReport,
      sendFeatureReport: p.sendFeatureReport,
      receiveFeatureReport: p.receiveFeatureReport,
    };

    p.sendReport = async function(reportId, data) {
      if (state.cur) state.cur.events.push({ dir: "send", kind: "report", reportId, hex: toHex(new Uint8Array(data)) });
      return state.orig.sendReport.apply(this, arguments);
    };

    p.sendFeatureReport = async function(reportId, data) {
      if (state.cur) state.cur.events.push({ dir: "send", kind: "feature", reportId, hex: toHex(new Uint8Array(data)) });
      return state.orig.sendFeatureReport.apply(this, arguments);
    };

    p.receiveFeatureReport = async function(reportId) {
      const out = await state.orig.receiveFeatureReport.apply(this, arguments);
      if (state.cur) state.cur.events.push({ dir: "recv", kind: "feature", reportId, hex: toHex(new Uint8Array(out.buffer || out)) });
      return out;
    };

    state.installed = true;
  }

  window.__hidTrace = {
    start(label) {
      install();
      if (state.cur) throw new Error("session already running");
      state.cur = { label, events: [] };
      return `started: ${label}`;
    },
    stop() {
      if (!state.cur) throw new Error("no active session");
      const s = state.cur;
      state.cur = null;
      state.sessions.push(s);
      return { label: s.label, events: s.events.length };
    },
    list() {
      return state.sessions.map((s, i) => ({ i, label: s.label, events: s.events.length }));
    },
    summary(i) {
      const s = state.sessions[i];
      const nonD2 = s.events.filter(e => !e.hex.startsWith("55 d2 "));
      const uniqueLines = [...new Set(nonD2.map(e => `${e.kind}|rid=${e.reportId}|${e.hex}`))];
      return { label: s.label, total: s.events.length, nonD2: nonD2.length, uniqueLines };
    },
  };
})();
```

### 2) Record one action per session

Example for brightness:

```js
__hidTrace.start("bri_00")
// set brightness 0% and click apply
__hidTrace.stop()

__hidTrace.start("bri_50")
// set brightness 50% and click apply
__hidTrace.stop()

__hidTrace.start("bri_100")
// set brightness 100% and click apply
__hidTrace.stop()
```

### 3) Diff the packets

```js
(() => {
  const idx = Object.fromEntries(__hidTrace.list().map(x => [x.label, x.i]));

  function bytes(label, packet = 0) {
    const lines = __hidTrace.summary(idx[label]).uniqueLines.filter(x => x.includes("|55 d6 "));
    const hex = lines[packet].split("|").pop().trim();
    return hex.split(" ").map(h => parseInt(h, 16));
  }

  function diff(a, b) {
    const out = [];
    for (let i = 0; i < Math.min(a.length, b.length); i++) {
      if (a[i] !== b[i]) out.push({ byte: i, a: a[i], b: b[i] });
    }
    return out;
  }

  const b0 = bytes("bri_00", 0);
  const b50 = bytes("bri_50", 0);
  const b100 = bytes("bri_100", 0);

  console.log("00 vs 50", diff(b0, b50));
  console.log("50 vs 100", diff(b50, b100));
  console.log("00 vs 100", diff(b0, b100));
})();
```

### 4) Implement in `nuphyctl`

1. Add packet constants/byte offsets in `src/main.rs`.
2. Encode parameter value to observed range.
3. Update checksum byte if needed (often byte `3` in `55 d6` packet).
4. Send set packet, then apply packet.
5. Validate with `raw send` before exposing a new CLI subcommand.

Tip: always keep mode/color fixed while testing one variable. Otherwise diffs include unrelated bytes.
