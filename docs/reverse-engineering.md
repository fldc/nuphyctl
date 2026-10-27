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

## Keymap protocol (implemented)

Recovered from the NuPhy IO web app (drive.nuphy.io) and verified on an Air75 V3 running
firmware 1.0.17.6.

Data commands use the same 64-byte framing as the lighting packets, with the length in
byte 4. `K` is the session key negotiated with `0xee` (with `K = 0` nothing is XORed):

```text
request:  55 cmd 00 sum  len^K addr_lo^K addr_hi^K 00^K  data^K ...
reply:    aa cmd 00 sum  len^K addr_lo^K addr_hi^K 00^K  data^K ...
```

- `sum` is the wrapping sum of bytes `4..63`.
- `addr` is a byte offset; `len` is at most 56 (64 minus the 8 header bytes).
- Replies echo `len`/`addr`; `nuphyctl` matches replies on command, `len` and `addr`.

Commands:

| cmd | name | request | reply |
| --- | --- | --- | --- |
| `0xa0` | GetBase | `len 8 addr 0` | `mode, layerCount, layersPerMode, rows, cols, extra` |
| `0xa1` | GetFirmwareInfo | `len 8 addr 0` | `b0 b1 b2 b3 b4 b5`: version `b2.b1.b0`, plus `.(b4 + 256*b5)` when `b3 == 0xaa` |
| `0xb1` | GetDefaultKeys | `len n addr a` | `n` bytes of the factory keymap |
| `0xb2` | GetUseKeys | `len n addr a` | `n` bytes of the active keymap |
| `0xb3` | SetUseKeys | `len n addr a data` | ack |
| `0xb4` | RestoreUseLayers | `len count addr first data = count zero bytes` | ack |

There is no save/commit command: `SetUseKeys` writes persist across replugs.
`RestoreUseLayers` copies the factory keymap over `count` layers starting at layer
`first` (for example `len 4 addr 4` resets the Windows layers 4-7).

`GetBase` on the Air75 V3 returns `01 08 04 06 10 82`: current mode Windows, 8 layers,
4 per mode, a 6x16 matrix and `0x82`. When bit 7 of the last byte is set, its low bits
give the number of extra (non-matrix) entries per layer (here 2); otherwise there are 2.

Keymap layout:

- Each layer is `2 * (rows * cols + extra)` bytes: 196 on the Air75 V3, 1568 for all 8 layers.
- Entry `i` of layer `L` is at byte `L * 196 + 2 * i`, a big-endian 16-bit keycode.
- Matrix keys come first (`i = row * 16 + col`), then the extras: `i = 96` knob
  counter-clockwise (factory `0x00aa` Volume Down), `i = 97` knob clockwise (factory
  `0x00a9` Volume Up).
- Layers `0-3` are Mac mode, `4-7` Windows mode; in each, base, Fn (`MO(1)`/`MO(5)` on
  the Fn key), and two further layers (factory: side-light controls on Fn+M and a Fn+X layer).

Air75 V3 positions: row 0 is Esc `(0,0)`, F1-F12 `(0,1..12)`, PrtSc `(0,13)`, the yellow
key `(0,14)` (factory Insert); `(0,15)` is empty. The red top-right key (the swappable knob
module) is `(3,12)` (factory Mute). `nuphyctl keymap positions` lists the rest.

Keycodes are QMK-style:

- `0x0000` no key, `0x0001` transparent.
- Basic HID usages `0x04-0xa4` and modifiers `0xe0-0xe7` as-is.
- QMK consumer/system block: `0xa8` Mute, `0xa9`/`0xaa` Volume Up/Down, `0xab-0xaf` media,
  `0xb0` Eject, `0xb1` Mail, `0xb2` Calculator, `0xb3` My Computer, `0xb4-0xba` browser keys,
  `0xbd`/`0xbe` screen brightness up/down, `0xd1-0xdc` mouse.
- `MO(n)` = `0x5220 | n`; QMK RGB codes `0x7820-0x782a`.
- NuPhy codes `0x7e00 + n`, e.g. `0x7e01-0x7e05` USB/2.4G/BT1-3, `0x7e06` Mission Control
  (Linux sees `KEY_SCALE`), `0x7e07` macOS search, `0x7e08` Siri (`KEY_VOICECOMMAND`),
  `0x7e09` Launchpad, `0x7e0a`/`0x7e0b` full/area screenshot, `0x7e16` Do Not Disturb
  (`KEY_DO_NOT_DISTURB`), `0x7e40`/`0x7e41` lock screen, `0x7e44` emoji.

`nuphyctl keymap keycodes` prints the full catalogue.

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
