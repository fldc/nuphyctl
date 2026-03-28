# NuPhy HID Reverse Engineering

This document captures the current protocol understanding behind `nuphyctl` and the repeatable workflow used to discover and validate new behavior.

## Scope

Use this workflow when adding controls like effect mode, speed, palette selection, per-key behavior, or other lighting parameters.

## Current protocol notes (implemented)

- Commands use 64-byte output reports.
- Frequent `55 d2 ...` packets in NuPhy web app are background polling.
- Before writes, web app performs key exchange using `55 ee ...`.
- Device responds with `aa ee ...`; bytes `4..=7` repeat the session key.
- Write packets (`55 d6` / `55 d5`) XOR-encode command payload bytes with the current session key.
- Checksum is packet byte `3`: wrapping sum of bytes `4..63`.

RGB flow used by this CLI:

1. Main light: `SET_DATA` (`0xd6`) subcommand `0x09` offset `0` payload `[effect, brightness, speed, direction, mode_flag, palette, r, g, b]`
2. Side light: `SET_DATA` (`0xd6`) subcommand `0x08` offset `9` payload `[effect, brightness, speed, mode_flag, palette, r, g, b]`
3. Decorative light currently reuses the side-light payload format with a model-dependent base offset
4. Brightness mirror writes use subcommand `0x01` at offsets `1` (main) and `10` (side)

Current CLI behavior:

- Main light uses legacy explicit apply (`0xd5`) for compatibility.
- Side and decorative writes currently skip explicit apply (matches observed GUI behavior in current web app traces).
- Write paths in `nuphyctl` retry a small set of transient HID failures before surfacing an error.

Main-light effect mapping captured from NuPhy Drive (Air75 V3, Lighting Effects list order):

- `1=Ray`
- `2=Stair`
- `3=Static`
- `4=Breath`
- `5=Flower`
- `6=Wave`
- `7=Ripple`
- `8=Spout`
- `9=Galaxy`
- `10=Rotation`
- `11=Ripple` (second entry in UI, exposed as `ripple2` in CLI)
- `12=Point`
- `13=Grid`
- `14=Time`
- `15=Rain`
- `16=Ribbon`
- `17=Gaming`
- `18=Identify`
- `19=Windmill`
- `20=Diagonal`

Observed payload nuance from GUI capture:

- In the newer legacy-addressed path (`sub=0x09`, `offset=0`), payload shape is `[effect, brightness, speed, direction, mode_flag, palette, r, g, b]`.
- `direction`: left=`1`, right=`0`.
- Main-light `mode_flag`: custom color=`1`, preset palette=`0`.
- Side-light `mode_flag`: custom color=`0`, preset palette=`1`.

Side-light effect mapping captured from NuPhy Drive:

- `0=Time`
- `1=Neon`
- `2=Static`
- `3=Breathe`
- `4=Rhythm`

Decorative/strip/front-light notes:

- Device families differ; the GUI bundle exposes both 2-channel and 3-channel layouts.
- Third channel offsets are model-dependent; `17` and `35` are known candidates from traces/bundle analysis.

Device targeting notes:

- `nuphyctl list` prints `path`, `vid`, `pid`, `iface`, `usage_page`, and `usage` so captures can be reproduced against the same HID interface.
- If no selector is passed, the CLI tries to auto-pick the most likely control interface.
- When multiple candidates remain, prefer `--path` for exact reproduction during packet experiments.

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
5. Validate with `cargo run -- raw send ...` or a narrow `cargo run -- rgb ... --path /dev/hidrawX` command before exposing new flags broadly.

## Practical notes

- Close NuPhy web app before running CLI writes; concurrent HID access can cause transient errors.
- Keep mode/color fixed when testing one variable to avoid noisy packet diffs.
- Prefer exact device selectors during reverse engineering so results are repeatable across reconnects and multiple interfaces.
