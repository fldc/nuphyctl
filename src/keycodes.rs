//! Keycode catalogue for the NuPhy keymap (`0xb1`/`0xb2`/`0xb3`) commands.
//!
//! Keycodes are QMK-style 16-bit values: basic HID usages and modifiers as-is,
//! consumer/system/mouse codes in QMK's `0xa5..=0xdf` block, `MO(n)` as
//! `0x5220 | n`, QMK RGB codes at `0x7820..=0x782a`, and NuPhy's own codes at
//! `0x7e00 + n`. Names and values were taken from the NuPhy IO web app
//! (drive.nuphy.io) keycode converter.

use anyhow::{Result, bail};

const MO_BASE: u16 = 0x5220;
const MO_MAX_LAYER: u16 = 0x1f;

#[derive(Clone, Copy, Debug)]
pub enum KeycodeGroup {
    Special,
    Basic,
    Modifier,
    Media,
    Mouse,
    Lighting,
    Nuphy,
}

impl KeycodeGroup {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Special => "special",
            Self::Basic => "basic",
            Self::Modifier => "modifier",
            Self::Media => "media",
            Self::Mouse => "mouse",
            Self::Lighting => "lighting",
            Self::Nuphy => "nuphy",
        }
    }
}

#[derive(Debug)]
pub struct Keycode {
    pub code: u16,
    /// Canonical name, printed by `keymap dump`.
    pub name: &'static str,
    /// Additional names accepted on input.
    pub aliases: &'static [&'static str],
    pub group: KeycodeGroup,
    pub description: &'static str,
}

macro_rules! kc {
    ($group:ident, $code:expr, $name:expr, [$($alias:expr),*], $desc:expr) => {
        Keycode {
            code: $code,
            name: $name,
            aliases: &[$($alias),*],
            group: KeycodeGroup::$group,
            description: $desc,
        }
    };
}

pub static KEYCODES: &[Keycode] = &[
    kc!(Special, 0x0000, "NO", ["NONE", "XXXXXXX"], "no key"),
    kc!(
        Special,
        0x0001,
        "TRNS",
        ["TRANSPARENT", "_______"],
        "transparent (falls through to the layer below)"
    ),
    kc!(Basic, 0x0004, "A", [], "A"),
    kc!(Basic, 0x0005, "B", [], "B"),
    kc!(Basic, 0x0006, "C", [], "C"),
    kc!(Basic, 0x0007, "D", [], "D"),
    kc!(Basic, 0x0008, "E", [], "E"),
    kc!(Basic, 0x0009, "F", [], "F"),
    kc!(Basic, 0x000a, "G", [], "G"),
    kc!(Basic, 0x000b, "H", [], "H"),
    kc!(Basic, 0x000c, "I", [], "I"),
    kc!(Basic, 0x000d, "J", [], "J"),
    kc!(Basic, 0x000e, "K", [], "K"),
    kc!(Basic, 0x000f, "L", [], "L"),
    kc!(Basic, 0x0010, "M", [], "M"),
    kc!(Basic, 0x0011, "N", [], "N"),
    kc!(Basic, 0x0012, "O", [], "O"),
    kc!(Basic, 0x0013, "P", [], "P"),
    kc!(Basic, 0x0014, "Q", [], "Q"),
    kc!(Basic, 0x0015, "R", [], "R"),
    kc!(Basic, 0x0016, "S", [], "S"),
    kc!(Basic, 0x0017, "T", [], "T"),
    kc!(Basic, 0x0018, "U", [], "U"),
    kc!(Basic, 0x0019, "V", [], "V"),
    kc!(Basic, 0x001a, "W", [], "W"),
    kc!(Basic, 0x001b, "X", [], "X"),
    kc!(Basic, 0x001c, "Y", [], "Y"),
    kc!(Basic, 0x001d, "Z", [], "Z"),
    kc!(Basic, 0x001e, "1", [], "1 !"),
    kc!(Basic, 0x001f, "2", [], "2 @"),
    kc!(Basic, 0x0020, "3", [], "3 #"),
    kc!(Basic, 0x0021, "4", [], "4 $"),
    kc!(Basic, 0x0022, "5", [], "5 %"),
    kc!(Basic, 0x0023, "6", [], "6 ^"),
    kc!(Basic, 0x0024, "7", [], "7 &"),
    kc!(Basic, 0x0025, "8", [], "8 *"),
    kc!(Basic, 0x0026, "9", [], "9 ("),
    kc!(Basic, 0x0027, "0", [], "0 )"),
    kc!(Basic, 0x0028, "ENT", ["ENTER"], "Enter"),
    kc!(Basic, 0x0029, "ESC", ["ESCAPE"], "Escape"),
    kc!(Basic, 0x002a, "BSPC", ["BACKSPACE"], "Backspace"),
    kc!(Basic, 0x002b, "TAB", [], "Tab"),
    kc!(Basic, 0x002c, "SPC", ["SPACE"], "Space"),
    kc!(Basic, 0x002d, "MINS", ["MINUS"], "- _"),
    kc!(Basic, 0x002e, "EQL", ["EQUAL"], "= +"),
    kc!(Basic, 0x002f, "LBRC", ["LEFT_BRACKET", "LBRACKET"], "[ {"),
    kc!(Basic, 0x0030, "RBRC", ["RIGHT_BRACKET", "RBRACKET"], "] }"),
    kc!(Basic, 0x0031, "BSLS", ["BACKSLASH"], "\\ |"),
    kc!(Basic, 0x0032, "NUHS", ["NONUS_HASH"], "non-US # ~"),
    kc!(Basic, 0x0033, "SCLN", ["SEMICOLON"], "; :"),
    kc!(Basic, 0x0034, "QUOT", ["QUOTE"], "' \""),
    kc!(Basic, 0x0035, "GRV", ["GRAVE"], "` ~"),
    kc!(Basic, 0x0036, "COMM", ["COMMA"], ", <"),
    kc!(Basic, 0x0037, "DOT", [], ". >"),
    kc!(Basic, 0x0038, "SLSH", ["SLASH"], "/ ?"),
    kc!(Basic, 0x0039, "CAPS", ["CAPS_LOCK"], "Caps Lock"),
    kc!(Basic, 0x003a, "F1", [], "F1"),
    kc!(Basic, 0x003b, "F2", [], "F2"),
    kc!(Basic, 0x003c, "F3", [], "F3"),
    kc!(Basic, 0x003d, "F4", [], "F4"),
    kc!(Basic, 0x003e, "F5", [], "F5"),
    kc!(Basic, 0x003f, "F6", [], "F6"),
    kc!(Basic, 0x0040, "F7", [], "F7"),
    kc!(Basic, 0x0041, "F8", [], "F8"),
    kc!(Basic, 0x0042, "F9", [], "F9"),
    kc!(Basic, 0x0043, "F10", [], "F10"),
    kc!(Basic, 0x0044, "F11", [], "F11"),
    kc!(Basic, 0x0045, "F12", [], "F12"),
    kc!(Basic, 0x0046, "PSCR", ["PRINT_SCREEN"], "Print Screen"),
    kc!(Basic, 0x0047, "SCRL", ["SCROLL_LOCK"], "Scroll Lock"),
    kc!(Basic, 0x0048, "PAUS", ["PAUSE"], "Pause"),
    kc!(Basic, 0x0049, "INS", ["INSERT"], "Insert"),
    kc!(Basic, 0x004a, "HOME", [], "Home"),
    kc!(Basic, 0x004b, "PGUP", ["PAGE_UP"], "Page Up"),
    kc!(Basic, 0x004c, "DEL", ["DELETE"], "Delete"),
    kc!(Basic, 0x004d, "END", [], "End"),
    kc!(Basic, 0x004e, "PGDN", ["PAGE_DOWN"], "Page Down"),
    kc!(Basic, 0x004f, "RGHT", ["RIGHT"], "Right arrow"),
    kc!(Basic, 0x0050, "LEFT", [], "Left arrow"),
    kc!(Basic, 0x0051, "DOWN", [], "Down arrow"),
    kc!(Basic, 0x0052, "UP", [], "Up arrow"),
    kc!(Basic, 0x0053, "NUM", ["NUM_LOCK"], "Num Lock"),
    kc!(Basic, 0x0054, "PSLS", ["KP_SLASH"], "keypad /"),
    kc!(Basic, 0x0055, "PAST", ["KP_ASTERISK"], "keypad *"),
    kc!(Basic, 0x0056, "PMNS", ["KP_MINUS"], "keypad -"),
    kc!(Basic, 0x0057, "PPLS", ["KP_PLUS"], "keypad +"),
    kc!(Basic, 0x0058, "PENT", ["KP_ENTER"], "keypad Enter"),
    kc!(Basic, 0x0059, "P1", ["KP_1"], "keypad 1"),
    kc!(Basic, 0x005a, "P2", ["KP_2"], "keypad 2"),
    kc!(Basic, 0x005b, "P3", ["KP_3"], "keypad 3"),
    kc!(Basic, 0x005c, "P4", ["KP_4"], "keypad 4"),
    kc!(Basic, 0x005d, "P5", ["KP_5"], "keypad 5"),
    kc!(Basic, 0x005e, "P6", ["KP_6"], "keypad 6"),
    kc!(Basic, 0x005f, "P7", ["KP_7"], "keypad 7"),
    kc!(Basic, 0x0060, "P8", ["KP_8"], "keypad 8"),
    kc!(Basic, 0x0061, "P9", ["KP_9"], "keypad 9"),
    kc!(Basic, 0x0062, "P0", ["KP_0"], "keypad 0"),
    kc!(Basic, 0x0063, "PDOT", ["KP_DOT"], "keypad ."),
    kc!(Basic, 0x0064, "NUBS", ["NONUS_BACKSLASH"], "non-US \\ |"),
    kc!(
        Basic,
        0x0065,
        "APP",
        ["APPLICATION", "MENU_KEY"],
        "Application / context menu"
    ),
    kc!(Basic, 0x0066, "KB_POWER", [], "keyboard Power"),
    kc!(Basic, 0x0067, "PEQL", ["KP_EQUAL"], "keypad ="),
    kc!(Basic, 0x0068, "F13", [], "F13"),
    kc!(Basic, 0x0069, "F14", [], "F14"),
    kc!(Basic, 0x006a, "F15", [], "F15"),
    kc!(Basic, 0x006b, "F16", [], "F16"),
    kc!(Basic, 0x006c, "F17", [], "F17"),
    kc!(Basic, 0x006d, "F18", [], "F18"),
    kc!(Basic, 0x006e, "F19", [], "F19"),
    kc!(Basic, 0x006f, "F20", [], "F20"),
    kc!(Basic, 0x0070, "F21", [], "F21"),
    kc!(Basic, 0x0071, "F22", [], "F22"),
    kc!(Basic, 0x0072, "F23", [], "F23"),
    kc!(Basic, 0x0073, "F24", [], "F24"),
    kc!(Basic, 0x0074, "EXEC", ["EXECUTE"], "Execute"),
    kc!(Basic, 0x0075, "HELP", [], "Help"),
    kc!(Basic, 0x0076, "MENU", [], "Menu"),
    kc!(Basic, 0x0077, "SLCT", ["SELECT"], "Select"),
    kc!(Basic, 0x0078, "STOP", [], "Stop"),
    kc!(Basic, 0x0079, "AGIN", ["AGAIN"], "Again"),
    kc!(Basic, 0x007a, "UNDO", [], "Undo"),
    kc!(Basic, 0x007b, "CUT", [], "Cut"),
    kc!(Basic, 0x007c, "COPY", [], "Copy"),
    kc!(Basic, 0x007d, "PSTE", ["PASTE"], "Paste"),
    kc!(Basic, 0x007e, "FIND", [], "Find"),
    kc!(Basic, 0x007f, "KB_MUTE", [], "keyboard-page Mute"),
    kc!(Basic, 0x0080, "KB_VOLUME_UP", [], "keyboard-page Volume Up"),
    kc!(
        Basic,
        0x0081,
        "KB_VOLUME_DOWN",
        [],
        "keyboard-page Volume Down"
    ),
    kc!(
        Basic,
        0x0082,
        "LCAP",
        ["LOCKING_CAPS_LOCK"],
        "locking Caps Lock"
    ),
    kc!(
        Basic,
        0x0083,
        "LNUM",
        ["LOCKING_NUM_LOCK"],
        "locking Num Lock"
    ),
    kc!(
        Basic,
        0x0084,
        "LSCR",
        ["LOCKING_SCROLL_LOCK"],
        "locking Scroll Lock"
    ),
    kc!(Basic, 0x0085, "PCMM", ["KP_COMMA"], "keypad ,"),
    kc!(Basic, 0x0086, "KP_EQUAL_AS400", [], "keypad = (AS/400)"),
    kc!(
        Basic,
        0x0087,
        "INT1",
        ["INTERNATIONAL_1", "RO"],
        "International 1 (JIS Ro)"
    ),
    kc!(
        Basic,
        0x0088,
        "INT2",
        ["INTERNATIONAL_2", "KANA"],
        "International 2 (Kana)"
    ),
    kc!(
        Basic,
        0x0089,
        "INT3",
        ["INTERNATIONAL_3", "JYEN"],
        "International 3 (Yen)"
    ),
    kc!(
        Basic,
        0x008a,
        "INT4",
        ["INTERNATIONAL_4", "HENK"],
        "International 4 (Henkan)"
    ),
    kc!(
        Basic,
        0x008b,
        "INT5",
        ["INTERNATIONAL_5", "MHEN"],
        "International 5 (Muhenkan)"
    ),
    kc!(
        Basic,
        0x008c,
        "INT6",
        ["INTERNATIONAL_6"],
        "International 6"
    ),
    kc!(
        Basic,
        0x008d,
        "INT7",
        ["INTERNATIONAL_7"],
        "International 7"
    ),
    kc!(
        Basic,
        0x008e,
        "INT8",
        ["INTERNATIONAL_8"],
        "International 8"
    ),
    kc!(
        Basic,
        0x008f,
        "INT9",
        ["INTERNATIONAL_9"],
        "International 9"
    ),
    kc!(
        Basic,
        0x0090,
        "LNG1",
        ["LANGUAGE_1", "HAEN"],
        "Language 1 (Han/Yeong)"
    ),
    kc!(
        Basic,
        0x0091,
        "LNG2",
        ["LANGUAGE_2", "HANJ"],
        "Language 2 (Hanja)"
    ),
    kc!(Basic, 0x0092, "LNG3", ["LANGUAGE_3"], "Language 3"),
    kc!(Basic, 0x0093, "LNG4", ["LANGUAGE_4"], "Language 4"),
    kc!(Basic, 0x0094, "LNG5", ["LANGUAGE_5"], "Language 5"),
    kc!(Basic, 0x0095, "LNG6", ["LANGUAGE_6"], "Language 6"),
    kc!(Basic, 0x0096, "LNG7", ["LANGUAGE_7"], "Language 7"),
    kc!(Basic, 0x0097, "LNG8", ["LANGUAGE_8"], "Language 8"),
    kc!(Basic, 0x0098, "LNG9", ["LANGUAGE_9"], "Language 9"),
    kc!(
        Basic,
        0x0099,
        "ERAS",
        ["ALTERNATE_ERASE"],
        "Alternate Erase"
    ),
    kc!(Basic, 0x009a, "SYRQ", ["SYSTEM_REQUEST"], "SysReq"),
    kc!(Basic, 0x009b, "CNCL", ["CANCEL"], "Cancel"),
    kc!(Basic, 0x009c, "CLR", ["CLEAR"], "Clear"),
    kc!(Basic, 0x009d, "PRIR", ["PRIOR"], "Prior"),
    kc!(Basic, 0x009e, "RETN", ["RETURN"], "Return"),
    kc!(Basic, 0x009f, "SEPR", ["SEPARATOR"], "Separator"),
    kc!(Basic, 0x00a0, "OUT", [], "Out"),
    kc!(Basic, 0x00a1, "OPER", [], "Oper"),
    kc!(Basic, 0x00a2, "CLAG", ["CLEAR_AGAIN"], "Clear/Again"),
    kc!(Basic, 0x00a3, "CRSL", ["CRSEL"], "CrSel"),
    kc!(Basic, 0x00a4, "EXSL", ["EXSEL"], "ExSel"),
    kc!(Media, 0x00a5, "PWR", ["SYSTEM_POWER"], "system Power"),
    kc!(Media, 0x00a6, "SLEP", ["SYSTEM_SLEEP"], "system Sleep"),
    kc!(Media, 0x00a7, "WAKE", ["SYSTEM_WAKE"], "system Wake"),
    kc!(Media, 0x00a8, "MUTE", ["AUDIO_MUTE"], "Mute"),
    kc!(
        Media,
        0x00a9,
        "VOLU",
        ["AUDIO_VOL_UP", "VOL_UP"],
        "Volume Up"
    ),
    kc!(
        Media,
        0x00aa,
        "VOLD",
        ["AUDIO_VOL_DOWN", "VOL_DOWN"],
        "Volume Down"
    ),
    kc!(
        Media,
        0x00ab,
        "MNXT",
        ["MEDIA_NEXT_TRACK", "MEDIA_NEXT"],
        "Next track"
    ),
    kc!(
        Media,
        0x00ac,
        "MPRV",
        ["MEDIA_PREV_TRACK", "MEDIA_PREV"],
        "Previous track"
    ),
    kc!(Media, 0x00ad, "MSTP", ["MEDIA_STOP"], "Stop media"),
    kc!(
        Media,
        0x00ae,
        "MPLY",
        ["MEDIA_PLAY_PAUSE", "PLAY"],
        "Play/Pause"
    ),
    kc!(
        Media,
        0x00af,
        "MSEL",
        ["MEDIA_SELECT", "MEDIA_PLAYER"],
        "Media player"
    ),
    kc!(Media, 0x00b0, "EJCT", ["EJECT", "MEDIA_EJECT"], "Eject"),
    kc!(Media, 0x00b1, "MAIL", [], "Mail"),
    kc!(Media, 0x00b2, "CALC", ["CALCULATOR"], "Calculator"),
    kc!(
        Media,
        0x00b3,
        "MYCM",
        ["MY_COMPUTER"],
        "My Computer / file browser"
    ),
    kc!(Media, 0x00b4, "WSCH", ["WWW_SEARCH"], "Browser search"),
    kc!(Media, 0x00b5, "WHOM", ["WWW_HOME"], "Browser home"),
    kc!(Media, 0x00b6, "WBAK", ["WWW_BACK"], "Browser back"),
    kc!(Media, 0x00b7, "WFWD", ["WWW_FORWARD"], "Browser forward"),
    kc!(Media, 0x00b8, "WSTP", ["WWW_STOP"], "Browser stop"),
    kc!(Media, 0x00b9, "WREF", ["WWW_REFRESH"], "Browser refresh"),
    kc!(
        Media,
        0x00ba,
        "WFAV",
        ["WWW_FAVORITES"],
        "Browser favorites"
    ),
    kc!(
        Media,
        0x00bb,
        "MFFD",
        ["MEDIA_FAST_FORWARD"],
        "Fast forward (not offered by NuPhy IO)"
    ),
    kc!(
        Media,
        0x00bc,
        "MRWD",
        ["MEDIA_REWIND"],
        "Rewind (not offered by NuPhy IO)"
    ),
    kc!(
        Media,
        0x00bd,
        "BRIU",
        ["BRIGHTNESS_UP"],
        "Screen brightness up"
    ),
    kc!(
        Media,
        0x00be,
        "BRID",
        ["BRIGHTNESS_DOWN"],
        "Screen brightness down"
    ),
    kc!(
        Media,
        0x00bf,
        "CPNL",
        ["CONTROL_PANEL"],
        "Control panel (not offered by NuPhy IO)"
    ),
    kc!(
        Media,
        0x00c0,
        "ASST",
        ["ASSISTANT"],
        "Assistant (not offered by NuPhy IO)"
    ),
    kc!(Mouse, 0x00cd, "MS_UP", [], "mouse cursor up"),
    kc!(Mouse, 0x00ce, "MS_DOWN", [], "mouse cursor down"),
    kc!(Mouse, 0x00cf, "MS_LEFT", [], "mouse cursor left"),
    kc!(Mouse, 0x00d0, "MS_RGHT", ["MS_RIGHT"], "mouse cursor right"),
    kc!(Mouse, 0x00d1, "BTN1", ["MS_BTN1"], "mouse left button"),
    kc!(Mouse, 0x00d2, "BTN2", ["MS_BTN2"], "mouse right button"),
    kc!(Mouse, 0x00d3, "BTN3", ["MS_BTN3"], "mouse middle button"),
    kc!(Mouse, 0x00d4, "BTN4", ["MS_BTN4"], "mouse back button"),
    kc!(Mouse, 0x00d5, "BTN5", ["MS_BTN5"], "mouse forward button"),
    kc!(Mouse, 0x00d9, "WH_U", ["MS_WH_UP"], "mouse wheel up"),
    kc!(Mouse, 0x00da, "WH_D", ["MS_WH_DOWN"], "mouse wheel down"),
    kc!(Mouse, 0x00db, "WH_L", ["MS_WH_LEFT"], "mouse wheel left"),
    kc!(Mouse, 0x00dc, "WH_R", ["MS_WH_RIGHT"], "mouse wheel right"),
    kc!(
        Modifier,
        0x00e0,
        "LCTL",
        ["LEFT_CTRL", "LCTRL"],
        "Left Ctrl"
    ),
    kc!(
        Modifier,
        0x00e1,
        "LSFT",
        ["LEFT_SHIFT", "LSHIFT"],
        "Left Shift"
    ),
    kc!(
        Modifier,
        0x00e2,
        "LALT",
        ["LEFT_ALT", "LOPT"],
        "Left Alt / Option"
    ),
    kc!(
        Modifier,
        0x00e3,
        "LGUI",
        ["LEFT_GUI", "LWIN", "LCMD"],
        "Left Win / Cmd"
    ),
    kc!(
        Modifier,
        0x00e4,
        "RCTL",
        ["RIGHT_CTRL", "RCTRL"],
        "Right Ctrl"
    ),
    kc!(
        Modifier,
        0x00e5,
        "RSFT",
        ["RIGHT_SHIFT", "RSHIFT"],
        "Right Shift"
    ),
    kc!(
        Modifier,
        0x00e6,
        "RALT",
        ["RIGHT_ALT", "ROPT", "ALGR"],
        "Right Alt / Option"
    ),
    kc!(
        Modifier,
        0x00e7,
        "RGUI",
        ["RIGHT_GUI", "RWIN", "RCMD"],
        "Right Win / Cmd"
    ),
    kc!(Lighting, 0x7820, "RGB_TOG", [], "backlight toggle"),
    kc!(
        Lighting,
        0x7821,
        "RGB_MOD",
        ["RGB_MODE_FORWARD"],
        "backlight next effect"
    ),
    kc!(
        Lighting,
        0x7822,
        "RGB_RMOD",
        ["RGB_MODE_REVERSE"],
        "backlight previous effect"
    ),
    kc!(Lighting, 0x7823, "RGB_HUI", [], "backlight hue up"),
    kc!(Lighting, 0x7824, "RGB_HUD", [], "backlight hue down"),
    kc!(Lighting, 0x7825, "RGB_SAI", [], "backlight saturation up"),
    kc!(Lighting, 0x7826, "RGB_SAD", [], "backlight saturation down"),
    kc!(Lighting, 0x7827, "RGB_VAI", [], "backlight brightness up"),
    kc!(Lighting, 0x7828, "RGB_VAD", [], "backlight brightness down"),
    kc!(Lighting, 0x7829, "RGB_SPI", [], "backlight speed up"),
    kc!(Lighting, 0x782a, "RGB_SPD", [], "backlight speed down"),
    kc!(
        Special,
        0x7c59,
        "QK_LOCK",
        ["KEYBOARD_LOCK"],
        "keyboard lock"
    ),
    kc!(Nuphy, 0x7e00, "BOOT", ["RF_DFU"], "enter bootloader"),
    kc!(
        Nuphy,
        0x7e01,
        "LINK_USB",
        ["LNK_USB", "USB"],
        "connection: USB"
    ),
    kc!(
        Nuphy,
        0x7e02,
        "LINK_24G",
        ["LNK_RF", "RF", "2_4G"],
        "connection: 2.4 GHz dongle"
    ),
    kc!(
        Nuphy,
        0x7e03,
        "BT1",
        ["LNK_BLE1", "BLE1"],
        "connection: Bluetooth 1"
    ),
    kc!(
        Nuphy,
        0x7e04,
        "BT2",
        ["LNK_BLE2", "BLE2"],
        "connection: Bluetooth 2"
    ),
    kc!(
        Nuphy,
        0x7e05,
        "BT3",
        ["LNK_BLE3", "BLE3"],
        "connection: Bluetooth 3"
    ),
    kc!(
        Nuphy,
        0x7e06,
        "MISSION_CONTROL",
        ["MAC_TASK"],
        "macOS Mission Control (Linux: KEY_SCALE)"
    ),
    kc!(
        Nuphy,
        0x7e07,
        "MAC_SEARCH",
        ["SPOTLIGHT"],
        "macOS search (sends Super+Space)"
    ),
    kc!(
        Nuphy,
        0x7e08,
        "SIRI",
        ["MAC_VOICE", "VOICE"],
        "Siri / voice command (Linux: KEY_VOICECOMMAND)"
    ),
    kc!(
        Nuphy,
        0x7e09,
        "LAUNCHPAD",
        ["MAC_CONSOLE"],
        "macOS Launchpad"
    ),
    kc!(
        Nuphy,
        0x7e0a,
        "SCREENSHOT",
        ["MAC_PRT", "SCREENSHOT_FULL"],
        "full-screen screenshot"
    ),
    kc!(
        Nuphy,
        0x7e0b,
        "SCREENSHOT_AREA",
        ["MAC_PRTA"],
        "area screenshot"
    ),
    kc!(
        Nuphy,
        0x7e0c,
        "MAC_GLOBE",
        ["MAC_GLOBOL", "GLOBE"],
        "macOS Globe key"
    ),
    kc!(Nuphy, 0x7e0d, "SIDE_VAI", [], "side light brightness up"),
    kc!(Nuphy, 0x7e0e, "SIDE_VAD", [], "side light brightness down"),
    kc!(Nuphy, 0x7e0f, "SIDE_MOD", [], "side light next mode"),
    kc!(Nuphy, 0x7e10, "SIDE_HUI", [], "side light next color"),
    kc!(Nuphy, 0x7e11, "SIDE_SPI", [], "side light speed up"),
    kc!(Nuphy, 0x7e12, "SIDE_SPD", [], "side light speed down"),
    kc!(
        Nuphy,
        0x7e13,
        "DEV_RESET",
        ["DEVICE_RESET"],
        "factory reset the keyboard"
    ),
    kc!(
        Nuphy,
        0x7e14,
        "SLEEP_TOGGLE",
        ["NP_SLEEP"],
        "toggle the keyboard sleep function"
    ),
    kc!(Nuphy, 0x7e15, "BATTERY", ["BAT_SHOW"], "show battery level"),
    kc!(
        Nuphy,
        0x7e16,
        "DND",
        ["MAC_DND", "DO_NOT_DISTURB"],
        "Do Not Disturb (Linux: KEY_DO_NOT_DISTURB)"
    ),
    kc!(Nuphy, 0x7e17, "BAT_NUM", [], "battery number (internal)"),
    kc!(
        Nuphy,
        0x7e18,
        "RGB_OFF",
        ["TOGGLE_RGB_SW"],
        "backlight on/off"
    ),
    kc!(Nuphy, 0x7e19, "RGB_TEST", [], "backlight test"),
    kc!(
        Nuphy,
        0x7e1a,
        "TOGGLE_DIRECTION",
        ["TOOGLE_DIRECTION"],
        "toggle direction (internal)"
    ),
    kc!(Nuphy, 0x7e1b, "LOOP_HUE", [], "backlight color loop"),
    kc!(Nuphy, 0x7e1c, "LOOP_LIGHT", [], "backlight brightness loop"),
    kc!(Nuphy, 0x7e1d, "LOOP_SPEED", [], "backlight speed loop"),
    kc!(
        Nuphy,
        0x7e1e,
        "LOCK_WIN",
        ["WIN_KEY_LOCK"],
        "lock the Win key"
    ),
    kc!(
        Nuphy,
        0x7e1f,
        "RGB_DIR_LEFT",
        [],
        "backlight direction left"
    ),
    kc!(
        Nuphy,
        0x7e20,
        "RGB_DIR_RIGHT",
        ["RGB_DEI_RIGHT"],
        "backlight direction right"
    ),
    kc!(
        Nuphy,
        0x7e21,
        "RGB_DIR_TOGGLE",
        [],
        "backlight direction toggle"
    ),
    kc!(
        Nuphy,
        0x7e22,
        "BLE_TEST_START",
        [],
        "factory test (internal)"
    ),
    kc!(
        Nuphy,
        0x7e23,
        "BLE_TEST_STOP",
        [],
        "factory test (internal)"
    ),
    kc!(
        Nuphy,
        0x7e24,
        "LOW_SLEEP_TEST",
        [],
        "factory test (internal)"
    ),
    kc!(
        Nuphy,
        0x7e25,
        "DEEP_SLEEP_TEST",
        [],
        "factory test (internal)"
    ),
    kc!(Nuphy, 0x7e26, "SIDE_TEST", [], "factory test (internal)"),
    kc!(Nuphy, 0x7e27, "DEBUG_STATUS", [], "debug status"),
    kc!(
        Nuphy,
        0x7e28,
        "PS_SHOW_OR_HIDE_RULERS",
        [],
        "Photoshop: show/hide rulers"
    ),
    kc!(
        Nuphy,
        0x7e29,
        "PS_NEW_DUPLICATE_LAYER",
        [],
        "Photoshop: duplicate layer"
    ),
    kc!(
        Nuphy,
        0x7e2a,
        "PS_MERGE_LAYERS",
        [],
        "Photoshop: merge layers"
    ),
    kc!(Nuphy, 0x7e2b, "PS_FILL", [], "Photoshop: fill"),
    kc!(
        Nuphy,
        0x7e2c,
        "PS_LEVELS_ADJUSTMENT",
        [],
        "Photoshop: levels"
    ),
    kc!(Nuphy, 0x7e2d, "PS_ZOOM_IN", [], "Photoshop: zoom in"),
    kc!(Nuphy, 0x7e2e, "PS_ZOOM_OUT", [], "Photoshop: zoom out"),
    kc!(
        Nuphy,
        0x7e2f,
        "AI_SAVE_FOR_WEB",
        [],
        "Illustrator: save for web"
    ),
    kc!(Nuphy, 0x7e30, "AI_UNDO", [], "Illustrator: undo"),
    kc!(
        Nuphy,
        0x7e31,
        "AI_SHOW_OR_HIDE_GUIDES",
        [],
        "Illustrator: show/hide guides"
    ),
    kc!(Nuphy, 0x7e32, "AI_ZOOM_IN", [], "Illustrator: zoom in"),
    kc!(Nuphy, 0x7e33, "AI_ZOOM_OUT", [], "Illustrator: zoom out"),
    kc!(
        Nuphy,
        0x7e34,
        "FIGMA_SHOW_OR_HIDE_UI",
        [],
        "Figma: show/hide UI"
    ),
    kc!(Nuphy, 0x7e35, "FIGMA_PIXEL_GRID", [], "Figma: pixel grid"),
    kc!(Nuphy, 0x7e36, "FIGMA_ALIGN_LEFT", [], "Figma: align left"),
    kc!(
        Nuphy,
        0x7e37,
        "FIGMA_FLATTEN_SELECTION",
        [],
        "Figma: flatten selection"
    ),
    kc!(
        Nuphy,
        0x7e38,
        "FIGMA_STRIKETHROUGH",
        [],
        "Figma: strikethrough"
    ),
    kc!(Nuphy, 0x7e39, "FIGMA_ZOOM_IN", [], "Figma: zoom in"),
    kc!(Nuphy, 0x7e3a, "FIGMA_ZOOM_OUT", [], "Figma: zoom out"),
    kc!(
        Nuphy,
        0x7e3b,
        "TOG_CAP_IND",
        [],
        "toggle Caps Lock indicator"
    ),
    kc!(Nuphy, 0x7e3c, "TOG_SIDE_LED", [], "toggle side light"),
    kc!(Nuphy, 0x7e3d, "FN_REVERSE", [], "swap Fn and media row"),
    kc!(Nuphy, 0x7e3e, "SIDE_SWITCH", [], "side light switch"),
    kc!(Nuphy, 0x7e3f, "MAC_SRT", [], "macOS SRT (unknown)"),
    kc!(
        Nuphy,
        0x7e40,
        "LOCK_SCREEN_WIN",
        ["WIN_LOCK_SCREEN"],
        "Windows: lock screen"
    ),
    kc!(
        Nuphy,
        0x7e41,
        "LOCK_SCREEN_MAC",
        ["MAC_LOCK_SCREEN"],
        "macOS: lock screen"
    ),
    kc!(
        Nuphy,
        0x7e42,
        "MAC_FINDER_SEARCH",
        [],
        "macOS: Finder search"
    ),
    kc!(Nuphy, 0x7e43, "MAC_FORCE_QUIT", [], "macOS: force quit"),
    kc!(Nuphy, 0x7e44, "EMOJI", ["MAC_KC_EMOJI"], "emoji picker"),
    kc!(
        Nuphy,
        0x7e45,
        "BACK_TO_DESKTOP",
        ["MAC_BACK_2_DESKTOP"],
        "back to desktop"
    ),
    kc!(
        Nuphy,
        0x7e46,
        "TASK_MANAGER",
        ["WIN_TASK_MANAGER"],
        "Windows: task manager"
    ),
    kc!(Nuphy, 0x7e47, "TOUCH_SWITCH", [], "touch bar switch"),
    kc!(
        Nuphy,
        0x7e48,
        "WIN_SWITCH_DESKTOPS_LEFT",
        [],
        "Windows: previous desktop"
    ),
    kc!(
        Nuphy,
        0x7e49,
        "WIN_SWITCH_DESKTOPS_RIGHT",
        [],
        "Windows: next desktop"
    ),
    kc!(
        Nuphy,
        0x7e4a,
        "MAC_SWITCH_DESKTOPS_LEFT",
        [],
        "macOS: previous desktop"
    ),
    kc!(
        Nuphy,
        0x7e4b,
        "MAC_SWITCH_DESKTOPS_RIGHT",
        [],
        "macOS: next desktop"
    ),
    kc!(
        Nuphy,
        0x7e4c,
        "WIN_TOUCH_PGM_LEFT",
        [],
        "Windows: touch program left"
    ),
    kc!(
        Nuphy,
        0x7e4d,
        "WIN_TOUCH_PGM_RIGHT",
        [],
        "Windows: touch program right"
    ),
    kc!(
        Nuphy,
        0x7e4e,
        "MAC_TOUCH_PGM_LEFT",
        [],
        "macOS: touch program left"
    ),
    kc!(
        Nuphy,
        0x7e4f,
        "MAC_TOUCH_PGM_RIGHT",
        [],
        "macOS: touch program right"
    ),
    kc!(
        Nuphy,
        0x7e50,
        "TASK_VIEW",
        ["CTRL_CENTER", "WIN_CTRL_CENTER"],
        "Windows: task view"
    ),
    kc!(Nuphy, 0x7e51, "WINDOW_WELT_LEFT", [], "snap window left"),
    kc!(Nuphy, 0x7e52, "WINDOW_WELT_RIGHT", [], "snap window right"),
    kc!(
        Nuphy,
        0x7e53,
        "SHOW_DESKTOP",
        ["WIN_SHOW_DESKTOP"],
        "Windows: show desktop"
    ),
    kc!(
        Nuphy,
        0x7e54,
        "MAC_LINGO_SWITCH",
        [],
        "macOS: switch input language"
    ),
    kc!(
        Nuphy,
        0x7e55,
        "DICTATE",
        ["WIN_DICTATE"],
        "Windows: dictation"
    ),
    kc!(Nuphy, 0x7e56, "FLEXO1", [], "Flexo 1"),
    kc!(
        Nuphy,
        0x7e57,
        "WIN_REPORT_REVERSE",
        [],
        "Windows: report reverse"
    ),
];

fn normalize(input: &str) -> String {
    let upper = input.trim().to_ascii_uppercase();
    match upper.strip_prefix("KC_") {
        Some(rest) if !rest.is_empty() => rest.to_string(),
        _ => upper,
    }
}

pub fn lookup_name(name: &str) -> Option<&'static Keycode> {
    let wanted = normalize(name);
    KEYCODES
        .iter()
        .find(|k| k.name == wanted || k.aliases.contains(&wanted.as_str()))
}

pub fn lookup_code(code: u16) -> Option<&'static Keycode> {
    KEYCODES.iter().find(|k| k.code == code)
}

/// Parse a keycode: a name from the catalogue (optionally `KC_`-prefixed,
/// case-insensitive), `MO(n)`, or a hex value like `0x00be`.
pub fn parse_keycode(input: &str) -> Result<u16> {
    let trimmed = input.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return u16::from_str_radix(hex, 16)
            .map_err(|_| anyhow::anyhow!("invalid hex keycode: {trimmed}"));
    }

    let upper = trimmed.to_ascii_uppercase();
    if let Some(inner) = upper
        .strip_prefix("MO(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let layer: u16 = inner
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid layer in {trimmed}"))?;
        if layer > MO_MAX_LAYER {
            bail!("layer out of range in {trimmed} (max {MO_MAX_LAYER})");
        }
        return Ok(MO_BASE | layer);
    }

    match lookup_name(trimmed) {
        Some(k) => Ok(k.code),
        None => bail!(
            "unknown keycode {trimmed:?}; see `nuphyctl keymap keycodes` or pass hex like 0x00be"
        ),
    }
}

/// Symbolic name for a keycode, or `None` if it is not in the catalogue.
pub fn keycode_name(code: u16) -> Option<String> {
    if let Some(k) = lookup_code(code) {
        return Some(k.name.to_string());
    }
    if code & !MO_MAX_LAYER == MO_BASE {
        return Some(format!("MO({})", code & MO_MAX_LAYER));
    }
    None
}

pub fn format_hex(code: u16) -> String {
    format!("0x{code:04x}")
}

/// Name when known, otherwise hex.
pub fn display_keycode(code: u16) -> String {
    keycode_name(code).unwrap_or_else(|| format_hex(code))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn names_and_aliases_are_unique() {
        let mut seen: HashMap<&str, u16> = HashMap::new();
        for k in KEYCODES {
            for name in std::iter::once(&k.name).chain(k.aliases.iter()) {
                assert_eq!(name.to_ascii_uppercase(), *name, "{name} not uppercase");
                if let Some(prev) = seen.insert(name, k.code) {
                    panic!("{name} maps to both {prev:#06x} and {:#06x}", k.code);
                }
            }
        }
    }

    #[test]
    fn codes_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for k in KEYCODES {
            assert!(seen.insert(k.code), "duplicate code {:#06x}", k.code);
        }
    }

    #[test]
    fn parses_names_hex_and_layers() {
        assert_eq!(parse_keycode("BRID").unwrap(), 0x00be);
        assert_eq!(parse_keycode("kc_brid").unwrap(), 0x00be);
        assert_eq!(parse_keycode("BRIGHTNESS_DOWN").unwrap(), 0x00be);
        assert_eq!(parse_keycode("F20").unwrap(), 0x006f);
        assert_eq!(parse_keycode("F1").unwrap(), 0x003a);
        assert_eq!(parse_keycode("F13").unwrap(), 0x0068);
        assert_eq!(parse_keycode("MISSION_CONTROL").unwrap(), 0x7e06);
        assert_eq!(parse_keycode("DND").unwrap(), 0x7e16);
        assert_eq!(parse_keycode("TRNS").unwrap(), 0x0001);
        assert_eq!(parse_keycode("_______").unwrap(), 0x0001);
        assert_eq!(parse_keycode("NO").unwrap(), 0x0000);
        assert_eq!(parse_keycode("MO(5)").unwrap(), 0x5225);
        assert_eq!(parse_keycode("mo( 1 )").unwrap(), 0x5221);
        assert_eq!(parse_keycode("0x7E0A").unwrap(), 0x7e0a);
        assert_eq!(parse_keycode(" 0x00be ").unwrap(), 0x00be);
        assert_eq!(parse_keycode("1").unwrap(), 0x001e);
        assert_eq!(parse_keycode("0").unwrap(), 0x0027);
        assert_eq!(parse_keycode("KC_0").unwrap(), 0x0027);
    }

    #[test]
    fn rejects_bad_keycodes() {
        assert!(parse_keycode("NOPE").is_err());
        assert!(parse_keycode("0xZZ").is_err());
        assert!(parse_keycode("0x10000").is_err());
        assert!(parse_keycode("MO(32)").is_err());
        assert!(parse_keycode("MO(x)").is_err());
        assert!(parse_keycode("KC_").is_err());
    }

    #[test]
    fn names_codes() {
        assert_eq!(display_keycode(0x00be), "BRID");
        assert_eq!(display_keycode(0x7e06), "MISSION_CONTROL");
        assert_eq!(display_keycode(0x5225), "MO(5)");
        assert_eq!(display_keycode(0x5221), "MO(1)");
        assert_eq!(display_keycode(0x1234), "0x1234");
        assert_eq!(display_keycode(0x0001), "TRNS");
    }

    #[test]
    fn display_round_trips() {
        for code in [
            0x0000u16, 0x0001, 0x0029, 0x00e7, 0x5223, 0x7821, 0x7e57, 0x4242,
        ] {
            assert_eq!(parse_keycode(&display_keycode(code)).unwrap(), code);
        }
    }
}
