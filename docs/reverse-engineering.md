# NuPhy HID Reverse Engineering

This document captures the current protocol understanding and the repeatable process used to discover/validate it.

## Scope

Use this workflow when adding controls like effect mode, speed, per-key behavior, or additional lighting parameters.

## Current protocol notes (implemented)

- Commands use 64-byte output reports.
- Frequent `55 d2 ...` packets in NuPhy web app are background polling.
- Before writes, web app performs key exchange using `55 ee ...`.
- Device responds with `aa ee ...`; bytes `4..=7` repeat the session key.
- Write packets (`55 d6` / `55 d5`) XOR-encode command payload bytes with the current session key.
- Checksum is packet byte `3`: wrapping sum of bytes `4..63`.

Static RGB flow used by this CLI:

1. `SET_DATA` (`0xd6`) subcommand `0x09` with payload `[effect, brightness, color_mode, 0, 0, 0, r, g, b]`
2. `SET_DATA` (`0xd6`) subcommand `0x01` for brightness
3. `APPLY` (`0xd5`) subcommand `0x11`

## 1) Capture packets in Chrome

Open <https://drive.nuphy.io/> in Chrome, then open DevTools Console and paste:

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
      const nonPoll = s.events.filter(e => !e.hex.startsWith("55 d2 "));
      const uniqueLines = [...new Set(nonPoll.map(e => `${e.kind}|rid=${e.reportId}|${e.hex}`))];
      return { label: s.label, total: s.events.length, nonPoll: nonPoll.length, uniqueLines };
    },
  };
})();
```

## 2) Record one action per trace session

Keep each session to one setting change so diffs stay isolated.

```js
__hidTrace.start("brightness_25")
// set brightness to 25 and apply
__hidTrace.stop()

__hidTrace.start("brightness_80")
// set brightness to 80 and apply
__hidTrace.stop()
```

## 3) Inspect and diff non-poll packets

```js
(() => {
  const idx = Object.fromEntries(__hidTrace.list().map(x => [x.label, x.i]));
  const a = __hidTrace.summary(idx["brightness_25"]);
  const b = __hidTrace.summary(idx["brightness_80"]);
  console.log("A", a.uniqueLines);
  console.log("B", b.uniqueLines);
})();
```

Look for:

- `55 ee ...` / `aa ee ...` (session key exchange)
- `55 d6 ...` writes with changed encoded bytes
- `55 d5 ...` apply packet

## 4) Implement in `nuphyctl`

1. Add protocol constants and packet logic in `src/nuphy_protocol.rs`.
2. Expose new input parameters in `src/cli.rs`.
3. Wire command flow in `src/app.rs`.
4. Reuse transport primitives in `src/hid_transport.rs`.
5. Validate with `cargo run -- raw send ...` before exposing new flags broadly.

## Practical notes

- Close NuPhy web app before running CLI writes; concurrent HID access can cause transient errors.
- Keep mode/color fixed when testing one variable to avoid noisy packet diffs.
