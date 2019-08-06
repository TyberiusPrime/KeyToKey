use alloc::{format, string::String};
use core::convert::{TryFrom, TryInto};
use num_enum::{IntoPrimitive, TryFromPrimitive};
pub const UNICODE_BELOW_256: u32 = 0x100000;
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
    H,
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
    Kp1,
    Kp2,
    Kp3,
    Kp4,
    Kp5,
    Kp6,
    Kp7,
    Kp8, // 0x60
    Kp9,
    Kp0,
    KpDot,
    NonUsBslash, // Non-US \ and | (Typically near the Left-Shift key)
    Application, // 0x65

    Power, // 0x66,
    KpEqual ,// 0x67, // Keypad =

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
    Props, /// 0x76,      // Keyboard Menu
    Front, // 0x77,      // Keyboard Select
    Stop, // 0x78,       // Keyboard Stop
    Again, // 0x79,      // Keyboard Again
    Undo, // 0x7a,       // Keyboard Undo
    Cut, // 0x7b,        // Keyboard Cut
    Copy, // 0x7c,       // Keyboard Copy
    Paste, // 0x7d,      // Keyboard Paste
    Find, // 0x7e,       // Keyboard Find
    Mute, // 0x7f,       // Keyboard Mute
    VolumeUp, // 0x80,   // Keyboard Volume Up
    VolumeDown, // 0x81, // Keyboard Volume Down
    // // 0x82  Keyboard Locking Caps Lock
    // // 0x83  Keyboard Locking Num Lock
    // // 0x84  Keyboard Locking Scroll Lock
    Kpcomma, // 0x85, // Keypad Comma
    // // 0x86  Keypad Equal Sign
    Ro, // 0x87,               // Keyboard International1
    Katakanahiragana, // 0x88, // Keyboard International2
    Yen, // 0x89,              // Keyboard International3
    Henkan, // 0x8a,           // Keyboard International4
    Muhenkan, // 0x8b,         // Keyboard International5
    KpJpComma, // 0x8c,        // Keyboard International6
    // // 0x8d  Keyboard International7
    // // 0x8e  Keyboard International8
    // // 0x8f  Keyboard International9
    Hangeul, // 0x90,        // Keyboard LANG1
    Hanja, // 0x91,          // Keyboard LANG2
    Katakana, // 0x92,       // Keyboard LANG3
    Hiragana, // 0x93,       // Keyboard LANG // 0x674
    Zenkakuhankaku, // 0x94, // Keyboard LANG5

    // Modifiers
    LCtrl = 0xE0 + UNICODE_BELOW_256,
    LShift,
    LAlt,
    LGui,
    RCtrl,
    RShift,
    RAlt,
    RGui, // 0xE7
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
}
impl TryFrom<u8> for KeyCode {
    type Error = String;
    fn try_from(ii: u8) -> Result<KeyCode, Self::Error> {
        let x: u32 = (ii as u32) + UNICODE_BELOW_256;
        return x.try_into();
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
