use alloc::{format, string::String};
use core::convert::{TryFrom, TryInto};
use num_enum::{IntoPrimitive, TryFromPrimitive};

// here because the external users will need it.
/// If you want to send a Unicode below id=256
/// you'll need to add this value, otherwise
/// you'll be sending standard USB keycodes instead.
pub const UNICODE_BELOW_256: u32 = 0x100000;


/// usb key codes mapped into the first private region of unicode
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
    Bslash,    // \ (and |)
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
    pub fn is_modifier(self) -> bool {
        KeyCode::LCtrl <= self && self <= KeyCode::RGui
    }
    pub fn as_modifier_bit(self) -> u8 {
        if self.is_modifier() {
            1 << (self.to_u8()- KeyCode::LCtrl.to_u8())
        } else {
            0
        }
    }

    pub fn to_u8(self) -> u8
    {
        let u = (self as u32) - UNICODE_BELOW_256;
        return u as u8;
    }
}

impl TryFrom<u8> for KeyCode {
    type Error = String;
    fn try_from(ii: u8) -> Result<KeyCode, Self::Error> {
        let x: u32 = (ii as u32)+ UNICODE_BELOW_256;
        return x.try_into();
    }
}

pub trait AcceptsKeycode {
    fn to_u32(&self) -> u32;
}

impl AcceptsKeycode for u32 {
    fn to_u32(&self) -> u32 {
        *self
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
