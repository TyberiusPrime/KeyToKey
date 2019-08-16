use alloc::{format, string::String};
use core::convert::{TryFrom, TryInto};
use num_enum::{IntoPrimitive, TryFromPrimitive};
pub const UNICODE_BELOW_256: u32 = 0x100_000;
/// usb key codes mapped into the first private region of unicode
/// USBKeyOut must substract UNICODE_BELOW_256 to create valid u8 values
/// to transmit
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, IntoPrimitive, TryFromPrimitive, Debug)]
#[repr(u32)]
pub enum KeyCode {
    No = UNICODE_BELOW_256,
    ErrorRollOver,
    PostFail,
    ErrorUndefined,
    A, // 4
    B,
    C,
    D,
    E,
    F,
    G,
    H, //11
    I,
    J,
    K,
    L,
    M, // 0x10
    N,
    O,
    P,
    Q, //20
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,   //29
    Kb1, // Keyboard 1 30
    Kb2,
    Kb3, // 0x20
    Kb4,
    Kb5,
    Kb6,
    Kb7,
    Kb8,
    Kb9,
    Kb0, //40
    Enter,
    Escape,
    BSpace,
    Tab,
    Space,
    Minus, //0x2D - 45
    Equal,
    LBracket,
    RBracket,  // 0x30 --48
    BSlash,    // \ (and |)
    NonUsHash, // Non-US # and ~ (Typically near the Enter key)
    SColon,    // ; (and :)
    Quote,     // ' and "
    Grave,     // Grave accent and tilde
    Comma,     // , and <
    Dot,       // . and >
    Slash,     // / and ?
    CapsLock,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7, // 0x40
    F8,
    F9,
    F10,
    F11,
    F12,
    PScreen,
    ScrollLock,
    Pause,
    Insert,
    Home,
    PgUp,
    Delete,
    End,
    PgDown,
    Right,
    Left, // 0x50
    Down,
    Up,
    NumLock,
    KpSlash,
    KpAsterisk,
    KpMinus,
    KpPlus,
    KpEnter,
    Kp1, //89
    Kp2,
    Kp3,
    Kp4,
    Kp5,
    Kp6, //94
    Kp7,
    Kp8, // 0x60 - 96
    Kp9,
    Kp0,
    KpDot,
    NonUsBslash, // Non-US \ and | (Typically near the Left-Shift key)
    Application, // 0x65

    Power,   // 0x66,
    KpEqual, // 0x67, // Keypad =

    F13, // 0x68 Keyboard F13
    F14, // 0x69, // Keyboard F14
    F15, // 0x6a, // Keyboard F15
    F16, // 0x6b, // Keyboard F16
    F17, // 0x6c, // Keyboard F17
    F18, // 0x6d, // Keyboard F18
    F19, // 0x6e, // Keyboard F19
    F20, // 0x6f, // Keyboard F20
    F21, // 0x70, // Keyboard F21
    F22, // 0x71, // Keyboard F22
    F23, // 0x72, // Keyboard F23
    F24, // 0x73, // Keyboard F24

    Open, // 0x74,       // Keyboard Execute
    Help, // 0x75,       // Keyboard Help
    Props,
    /// 0x76,      // Keyboard Menu
    Front, // 0x77,      // Keyboard Select
    Stop,       // 0x78,       // Keyboard Stop
    Again,      // 0x79,      // Keyboard Again
    Undo,       // 0x7a,       // Keyboard Undo
    Cut,        // 0x7b,        // Keyboard Cut
    Copy,       // 0x7c,       // Keyboard Copy
    Paste,      // 0x7d,      // Keyboard Paste
    Find,       // 0x7e,       // Keyboard Find
    Mute,       // 0x7f,       // Keyboard Mute
    VolumeUp,   // 0x80,   // Keyboard Volume Up
    VolumeDown, // 0x81, // Keyboard Volume Down
    // // 0x82  Keyboard Locking Caps Lock
    // // 0x83  Keyboard Locking Num Lock
    // // 0x84  Keyboard Locking Scroll Lock
    Kpcomma, // 0x85, // Keypad Comma
    // // 0x86  Keypad Equal Sign
    Ro,               // 0x87,               // Keyboard International1
    Katakanahiragana, // 0x88, // Keyboard International2
    Yen,              // 0x89,              // Keyboard International3
    Henkan,           // 0x8a,           // Keyboard International4
    Muhenkan,         // 0x8b,         // Keyboard International5
    KpJpComma,        // 0x8c,        // Keyboard International6
    // // 0x8d  Keyboard International7
    // // 0x8e  Keyboard International8
    // // 0x8f  Keyboard International9
    Hangeul,        // 0x90,        // Keyboard LANG1
    Hanja,          // 0x91,          // Keyboard LANG2
    Katakana,       // 0x92,       // Keyboard LANG3
    Hiragana,       // 0x93,       // Keyboard LANG // 0x674
    Zenkakuhankaku, // 0x94, // Keyboard LANG5

    KpLeftParen = 0xb6 + UNICODE_BELOW_256, // Keypad (
    KpRightParen,                           // 0xb7 // Keypad )
    // Modifiers
    LCtrl = 0xE0 + UNICODE_BELOW_256, //224
    LShift, //225
    LAlt,
    LGui,
    RCtrl,
    RShift,
    RAlt,
    RGui, // 0xE7

    MediaPlayPause = 0xE8 + UNICODE_BELOW_256,
    MediaStopCd,
    MediaPrevioussong,
    MediaNextsong,
    MediaEjectCd,
    MediaVolumeUp,
    MediaVolumeDown,
    MediaMUte,
    MediaWww,
    MediaBack,
    MediaForward,
    MediaStop,
    MediaFind,
    MediaScrollUp,
    MediaScrollDown,
    MediaEdit,
    MediaSleep,
    MediaCoffee,
    MediaRefresh,
    MediaCalc,
}
impl KeyCode {
    /// needed to build USB reports
    pub fn is_modifier(self) -> bool {
        KeyCode::LCtrl <= self && self <= KeyCode::RGui
    }
    /// needed to build USB reports
    pub fn as_modifier_bit(self) -> u8 {
        if self.is_modifier() {
            1 << (self.to_u8() - KeyCode::LCtrl.to_u8())
        } else {
            0
        }
    }
    pub fn to_u8(self) -> u8 {
        let u = (self as u32) - UNICODE_BELOW_256;
        return u as u8;
    }

    pub const fn to_u32(self) -> u32 {
        let u = self as u32;
        return u as u32;
    }
}
impl TryFrom<u8> for KeyCode {
    type Error = String;
    fn try_from(ii: u8) -> Result<KeyCode, Self::Error> {
        let x: u32 = u32::from(ii) + UNICODE_BELOW_256;
        return x.try_into();
    }
}
/// KeyCodes not being used by anything by default
/// so you're free to use these to assign macros/tapdances/leaders
/// and what not.
#[repr(u32)]
#[derive(IntoPrimitive, Copy, Clone)]
pub enum UserKey {
    UK0 = 0xF0100,
    UK1 = 0xF0101,
    UK2 = 0xF0102,
    UK3 = 0xF0103,
    UK4 = 0xF0104,
    UK5 = 0xF0105,
    UK6 = 0xF0106,
    UK7 = 0xF0107,
    UK8 = 0xF0108,
    UK9 = 0xF0109,
    UK10 = 0xF010A,
    UK11 = 0xF010B,
    UK12 = 0xF010C,
    UK13 = 0xF010D,
    UK14 = 0xF010E,
    UK15 = 0xF010F,
    UK16 = 0xF0110,
    UK17 = 0xF0111,
    UK18 = 0xF0112,
    UK19 = 0xF0113,
    UK20 = 0xF0114,
    UK21 = 0xF0115,
    UK22 = 0xF0116,
    UK23 = 0xF0117,
    UK24 = 0xF0118,
    UK25 = 0xF0119,
    UK26 = 0xF011A,
    UK27 = 0xF011B,
    UK28 = 0xF011C,
    UK29 = 0xF011D,
    UK30 = 0xF011E,
    UK31 = 0xF011F,
    UK32 = 0xF0120,
    UK33 = 0xF0121,
    UK34 = 0xF0122,
    UK35 = 0xF0123,
    UK36 = 0xF0124,
    UK37 = 0xF0125,
    UK38 = 0xF0126,
    UK39 = 0xF0127,
    UK40 = 0xF0128,
    UK41 = 0xF0129,
    UK42 = 0xF012A,
    UK43 = 0xF012B,
    UK44 = 0xF012C,
    UK45 = 0xF012D,
    UK46 = 0xF012E,
    UK47 = 0xF012F,
    UK48 = 0xF0130,
    UK49 = 0xF0131,
    UK50 = 0xF0132,
    UK51 = 0xF0133,
    UK52 = 0xF0134,
    UK53 = 0xF0135,
    UK54 = 0xF0136,
    UK55 = 0xF0137,
    UK56 = 0xF0138,
    UK57 = 0xF0139,
    UK58 = 0xF013A,
    UK59 = 0xF013B,
    UK60 = 0xF013C,
    UK61 = 0xF013D,
    UK62 = 0xF013E,
    UK63 = 0xF013F,
    UK64 = 0xF0140,
    UK65 = 0xF0141,
    UK66 = 0xF0142,
    UK67 = 0xF0143,
    UK68 = 0xF0144,
    UK69 = 0xF0145,
    UK70 = 0xF0146,
    UK71 = 0xF0147,
    UK72 = 0xF0148,
    UK73 = 0xF0149,
    UK74 = 0xF014A,
    UK75 = 0xF014B,
    UK76 = 0xF014C,
    UK77 = 0xF014D,
    UK78 = 0xF014E,
    UK79 = 0xF014F,
    UK80 = 0xF0150,
    UK81 = 0xF0151,
    UK82 = 0xF0152,
    UK83 = 0xF0153,
    UK84 = 0xF0154,
    UK85 = 0xF0155,
    UK86 = 0xF0156,
    UK87 = 0xF0157,
    UK88 = 0xF0158,
    UK89 = 0xF0159,
    UK90 = 0xF015A,
    UK91 = 0xF015B,
    UK92 = 0xF015C,
    UK93 = 0xF015D,
    UK94 = 0xF015E,
    UK95 = 0xF015F,
    UK96 = 0xF0160,
    UK97 = 0xF0161,
    UK98 = 0xF0162,
    UK99 = 0xF0163,
}

impl UserKey {
    pub const fn to_u32(self) -> u32 {
        let u = self as u32;
        return u as u32;
    }
}

/// Trait for things that can be converted to a u32 keycode
/// ie. various integers and (usb) KeyCodes themselves
pub trait AcceptsKeycode {
    fn to_u32(&self) -> u32;
}
impl AcceptsKeycode for u32 {
    fn to_u32(&self) -> u32 {
        *self
    }
}
impl AcceptsKeycode for &u32 {
    fn to_u32(&self) -> u32 {
        **self
    }
}
impl AcceptsKeycode for i32 {
    fn to_u32(&self) -> u32 {
        (*self) as u32
    }
}
impl AcceptsKeycode for KeyCode {
    fn to_u32(&self) -> u32 {
        let r: u32 = (*self).into();
        r
    }
}
impl AcceptsKeycode for UserKey {
    fn to_u32(&self) -> u32 {
        let r: u32 = (*self).into();
        r
    }
}
impl AcceptsKeycode for &UserKey {
    fn to_u32(&self) -> u32 {
        let r: u32 = (**self).into();
        r
    }
}

pub trait KeyCodeInfo {
    fn is_usb_keycode(self) -> bool;
    fn is_private_keycode(self) -> bool;
}

impl KeyCodeInfo for u32 {
    fn is_usb_keycode(self) -> bool {
        return UNICODE_BELOW_256 <= self && self <= UNICODE_BELOW_256 + 0xE7; //RGui
    }
    fn is_private_keycode(self) -> bool {
        return UserKey::UK0.to_u32() <= self && self <= UserKey::UK99.to_u32(); //RGui
    }
}
