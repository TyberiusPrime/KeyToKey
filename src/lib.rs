#![allow(dead_code)]
#![feature(drain_filter)]
#![no_std]
#![allow(clippy::needless_return, clippy::unreadable_literal)]
pub mod debug_handlers;
pub mod handlers;
mod key_codes;
mod key_stream;
pub mod premade;
pub mod test_helpers;
extern crate alloc;
extern crate no_std_compat;
extern crate spin;
pub use crate::handlers::{HandlerResult, ProcessKeys};

pub use crate::key_codes::{AcceptsKeycode, KeyCode, UserKey};
use crate::key_stream::Key;
pub use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use core::convert::TryInto;
use no_std_compat::prelude::v1::*;
use smallbitvec::{sbvec, SmallBitVec};

/// current keyboard state.
///
///
#[repr(u8)]
#[derive(PartialEq, Eq, Hash, Copy, Clone)]
pub enum Modifier {
    Shift = 0,
    Ctrl = 1,
    Alt = 2,
    Gui = 3,
}

const KEYBOARD_STATE_RESERVED_BITS: usize = 5;
const ABORT_BIT: usize = 4;

#[derive(Debug, Default)]
pub struct KeyboardState {
    pub unicode_mode: UnicodeSendMode,
    modifiers_and_enabled_handlers: SmallBitVec,
}
impl KeyboardState {
    pub fn new() -> KeyboardState {
        KeyboardState {
            unicode_mode: UnicodeSendMode::Linux,
            modifiers_and_enabled_handlers: sbvec![false; KEYBOARD_STATE_RESERVED_BITS],
        }
    }

    pub fn modifier(&self, modifier: Modifier) -> bool {
        self.modifiers_and_enabled_handlers[modifier as usize]
    }

    pub fn set_modifier(&mut self, modifier: Modifier, value: bool) {
        self.modifiers_and_enabled_handlers
            .set(modifier as usize, value);
    }

    pub fn enable_handler(&mut self, no: HandlerID) {
        self.modifiers_and_enabled_handlers.set(no, true);
    }

    pub fn disable_handler(&mut self, no: HandlerID) {
        self.modifiers_and_enabled_handlers.set(no, false);
    }

    pub fn set_handler(&mut self, no: HandlerID, enabled: bool) {
        self.modifiers_and_enabled_handlers.set(no, enabled);
    }

    pub fn toggle_handler(&mut self, no: HandlerID) {
        self.modifiers_and_enabled_handlers
            .set(no, !self.modifiers_and_enabled_handlers[no]);
    }

    pub fn is_handler_enabled(&self, no: HandlerID) -> bool {
        self.modifiers_and_enabled_handlers[no]
    }

    ///tell the Keyboard to
    /// * reset handlers to their default state, clear
    /// * clear all remaining events - unhandled or not
    /// * reset all modifiers to default
    pub fn abort_and_clear_events(&mut self) {
        self.modifiers_and_enabled_handlers.set(ABORT_BIT, true); // signal the handle_events loop to abort
    }

    fn _clear_abort(&mut self) {
        self.modifiers_and_enabled_handlers.set(ABORT_BIT, false);
    }

    fn _aborted(&self) -> bool {
        return self.modifiers_and_enabled_handlers[ABORT_BIT];
    }
}
///an identifer for an added handler
/// to be used with Keyboard.output.enable_handler and consorts
pub type HandlerID = usize;
/// the main keyboard struct
///
/// add handlers wit add_handler,
/// then call add_keypress/add_key_release/add_timeout
/// to start processing keys.
pub struct Keyboard<'a, T: USBKeyOut> {
    events: Vec<(Event, EventStatus)>,
    running_number: u8,
    handlers: Vec<Box<dyn ProcessKeys<T> + Send + 'a>>,
    pub output: T,
}
#[allow(clippy::new_without_default)]
impl<'a, T: USBKeyOut> Keyboard<'a, T> {
    pub fn new(output: T) -> Keyboard<'a, T> {
        Keyboard {
            events: Vec::new(),
            running_number: 0,
            handlers: Vec::new(),
            output,
        }
    }
    /// add a handler, return a HandlerID
    /// which you may use with keyboard.output.state().enable_handler / disable_handler / toggle_handler / is_handler_enabled
    ///
    /// by default, most handlers start in the enabled state (with the notable exception of Layers).
    pub fn add_handler(&mut self, handler: Box<dyn ProcessKeys<T> + Send + 'a>) -> HandlerID {
        self.output
            .state()
            .modifiers_and_enabled_handlers
            .push(handler.default_enabled());
        self.handlers.push(handler);
        return self.output.state().modifiers_and_enabled_handlers.len() - 1;
    }

    /// predict the next or further out hander_ids returned by add_handler
    /// Needed to add space cadets before the layers they toggle.
    pub fn future_handler_id(&self, offset: usize) -> HandlerID {
        let current = self.output.ro_state().modifiers_and_enabled_handlers.len() - 1;
        current + offset
    }

    /// handle an update to the event stream
    ///
    /// This returns OK(()) if all keys are handled by the handlers
    /// and an Err(()) otherwise.
    /// that way the down stream can decide what to do
    /// (tests: panic. Firmare/MatrixToStream -> drop unhandled events)
    pub fn handle_keys(&mut self) -> Result<(), ()> {
        for (_e, status) in self.events.iter_mut() {
            *status = EventStatus::Unhandled;
        }
        //skip the modifiers
        for (ii, h) in self.handlers.iter_mut().enumerate() {
            if self.output.state().modifiers_and_enabled_handlers[ii + KEYBOARD_STATE_RESERVED_BITS]
            {
                match h.process_keys(&mut self.events, &mut self.output) {
                    HandlerResult::NoOp => {}
                    HandlerResult::Disable => {
                        self.output
                            .state()
                            .disable_handler((ii + KEYBOARD_STATE_RESERVED_BITS) as HandlerID);
                    }
                }
                if self.output.state()._aborted() {
                    self.output.state()._clear_abort();
                    self.events.clear();
                    break; // no more handlers being done
                }
            }
        }
        // remove handled & timeout events.
        self.events.drain_filter(|(event, status)| {
            (EventStatus::Handled == *status)
                || (match event {
                    Event::TimeOut(_) => true,
                    _ => false,
                })
        });
        if self
            .events
            .iter()
            .any(|(_e, status)| EventStatus::Unhandled == *status)
        {
            return Err(());
        }
        Ok(())
    }
    //throw away unhandled key events
    pub fn clear_unhandled(&mut self) {
        self.events
            .drain_filter(|(_event, status)| (EventStatus::Unhandled == *status));
    }
    /// add a KeyPress event
    pub fn add_keypress<X: AcceptsKeycode>(&mut self, keycode: X, ms_since_last: u16) {
        let e = Key {
            keycode: keycode.to_u32(),
            original_keycode: keycode.to_u32(),
            ms_since_last,
            running_number: self.running_number,
            flag: 0,
        };
        self.running_number += 1;
        self.events
            .push((Event::KeyPress(e), EventStatus::Unhandled));
    }
    /// add a KeyRelease event
    pub fn add_keyrelease<X: AcceptsKeycode>(&mut self, keycode: X, ms_since_last: u16) {
        let e = Key {
            keycode: keycode.to_u32(),
            original_keycode: keycode.to_u32(),
            ms_since_last,
            running_number: self.running_number,
            flag: 0,
        };
        self.running_number += 1;
        self.events
            .push((Event::KeyRelease(e), EventStatus::Unhandled));
    }
    pub fn add_timeout(&mut self, ms_since_last: u16) {
        if let Some((event, _status)) = self.events.iter().last() {
            if let Event::TimeOut(_) = event {
                self.events.pop();
            }
        }
        self.events
            .push((Event::TimeOut(ms_since_last), EventStatus::Unhandled));
    }
}
/// Different operating systems expect random unicode input
/// as different key combinations
/// unfortunatly, we can't detect what we're connected to,
/// so the keyboard needs to provide some kinde of switch key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnicodeSendMode {
    //default X
    Linux = 1,
    LinuxDvorak,
    /// use https://github.com/samhocevar/wincompose
    WinCompose,
    WinComposeDvorak,
    // used by the tests
    Debug,
}
impl Default for UnicodeSendMode {
    fn default() -> UnicodeSendMode {
        UnicodeSendMode::Linux
    }
}
/// transform hex digits to USB keycodes
/// used by the unicode senders
fn hex_digit_to_keycode(digit: char) -> KeyCode {
    //todo which way it's shorter in machine code this or
    //with the derived nums...
    match digit {
        '0' => KeyCode::Kb0,
        '1' => KeyCode::Kb1,
        '2' => KeyCode::Kb2,
        '3' => KeyCode::Kb3,
        '4' => KeyCode::Kb4,
        '5' => KeyCode::Kb5,
        '6' => KeyCode::Kb6,
        '7' => KeyCode::Kb7,
        '8' => KeyCode::Kb8,
        '9' => KeyCode::Kb9,
        'A' | 'a' => KeyCode::A,
        'B' | 'b' => KeyCode::B,
        'C' | 'c' => KeyCode::C,
        'D' | 'd' => KeyCode::D,
        'E' | 'e' => KeyCode::E,
        'F' | 'f' => KeyCode::F,
        _ => panic!("Passed more than one digit to hex_digit_to_keycode"),
    }
}
fn hex_digit_to_keycode_dvorak(digit: char) -> KeyCode {
    //todo which way it's shorter in machine code this or
    //with the derived nums...
    match digit {
        '0' => KeyCode::Kb0,
        '1' => KeyCode::Kb1,
        '2' => KeyCode::Kb2,
        '3' => KeyCode::Kb3,
        '4' => KeyCode::Kb4,
        '5' => KeyCode::Kb5,
        '6' => KeyCode::Kb6,
        '7' => KeyCode::Kb7,
        '8' => KeyCode::Kb8,
        '9' => KeyCode::Kb9,
        'A' | 'a' => KeyCode::A,
        'B' | 'b' => KeyCode::N,
        'C' | 'c' => KeyCode::I,
        'D' | 'd' => KeyCode::H,
        'E' | 'e' => KeyCode::D,
        'F' | 'f' => KeyCode::Y,
        _ => panic!("Passed more than one digit to hex_digit_to_keycode"),
    }
}

/// the handlers use this trait to generate their output
pub trait USBKeyOut {
    /// send these USB Keycodes concurrently rigth away.
    fn send_keys(&mut self, keys: &[KeyCode]);
    /// register these USB keycodes to be send on .send_registered
    fn register_key(&mut self, key: KeyCode);
    /// send registered keycodes (or an empty nothing-pressed status)
    fn send_registered(&mut self);
    /// helper that sends an empty status
    fn send_empty(&mut self);
    /// retrieve a mutable KeyboardState
    fn state(&mut self) -> &mut KeyboardState;
    fn ro_state(&self) -> &KeyboardState;
    fn debug(&mut self, s: &str);
    fn bootloader(&mut self); // start the boot loader
    //
    // register to send later.
    fn send_keys_later(&mut self, keys: &[KeyCode], ms: u16);
    fn do_send_later(&mut self);

    fn send_unicode(&mut self, c: char) {
        match self.state().unicode_mode {
            UnicodeSendMode::Linux => {
                self.send_keys(&[KeyCode::LCtrl, KeyCode::LShift, KeyCode::U]);
                self.send_empty();
                for out_c in c.escape_unicode().skip(3).take_while(|x| *x != '}') {
                    self.send_keys(&[hex_digit_to_keycode(out_c)]);
                    self.send_empty();
                }
                self.send_keys(&[KeyCode::Enter]);
                self.send_empty();
            }
            UnicodeSendMode::LinuxDvorak => {
                self.send_keys(&[KeyCode::LCtrl, KeyCode::LShift, KeyCode::F]);
                self.send_empty();
                for _ in 0..10 {
                    //must be alternating
                    self.send_keys(&[KeyCode::LCtrl]);
                    self.send_empty();
                }
                for out_c in c.escape_unicode().skip(3).take_while(|x| *x != '}') {
                    self.send_keys(&[hex_digit_to_keycode_dvorak(out_c)]);
                    self.send_empty();
                    /* for _ in 0..10 {
                        //must be alternating
                        self.send_keys(&[KeyCode::LCtrl]);
                        self.send_empty();
                    } */
                }
                self.send_keys(&[KeyCode::Enter]);
                self.send_empty();
            }
            UnicodeSendMode::WinCompose => {
                self.send_keys(&[KeyCode::RAlt]);
                self.send_keys(&[KeyCode::U]);
                let escaped = c.escape_unicode();
                for out_c in escaped.skip(3).take_while(|x| *x != '}') {
                    self.send_keys(&[hex_digit_to_keycode(out_c)]);
                }
                self.send_keys(&[KeyCode::Enter]);
                self.send_empty();
            }
            UnicodeSendMode::WinComposeDvorak => {
                self.send_keys(&[KeyCode::RAlt]);
                self.send_keys(&[KeyCode::F]);
                let escaped = c.escape_unicode();
                for out_c in escaped.skip(3).take_while(|x| *x != '}') {
                    self.send_keys(&[hex_digit_to_keycode_dvorak(out_c)]);
                }
                self.send_keys(&[KeyCode::Enter]);
                self.send_empty();
            }

            UnicodeSendMode::Debug => {
                let escaped = c.escape_unicode();
                for out_c in escaped.skip(3).take_while(|x| *x != '}') {
                    self.send_keys(&[hex_digit_to_keycode(out_c)]);
                }
                //let mut buf = [0, 0, 0, 0];
                //c.encode_utf8(&mut buf);
                //self.send_keys(&[(u32::from(buf[0]) + UNICODE_BELOW_256).try_into().unwrap()]);
            }
        }
    }
    /// send a utf-8 string to the host
    /// all characters are converted into unicode input!
    fn send_string(&mut self, s: &str) {
        for c in s.chars() {
            /* the problem with this approach: it is dependant on the shift/caps lock state
            match c {
                'a'..='z' => {self.send_keys(&[ascii_to_keycode(c, 97, KeyCode::A)]); self.send_empty()},
                '1'..='9' => {self.send_keys(&[ascii_to_keycode(c, 49, KeyCode::Kb1)]); self.send_empty()},
                '0' => {self.send_keys(&[KeyCode::Kb0]); self.send_empty()},
                'A'..='Z' => {self.send_keys(&[ascii_to_keycode(c, 65, KeyCode::A), KeyCode::LShift]); self.send_empty()},
                _ => self.send_unicode(c),
            }
            */
            //probably best to unicode everything
            self.send_unicode(c);

            // option: send simple ones directly?
            /*
            if 'a' <= c && c <= 'z' {
                            }
            */
        }
    }
}
fn ascii_to_keycode(c: char, ascii_offset: u8, keycode_offset: KeyCode) -> KeyCode {
    let mut ascii = [0 as u8]; // buffer
    c.encode_utf8(&mut ascii);
    let keycode: u32 = keycode_offset.to_u32();
    let keycode = keycode as u8;
    let keycode = keycode + (ascii[0] - ascii_offset);
    let keycode: KeyCode = keycode.try_into().unwrap();
    keycode
}
//so the tests 'just work'.
#[cfg(test)]
#[macro_use]
extern crate std;

mod tests {
    #[test]
    fn test_hexdigit_to_keycode() {
        for c in "ABCDEFHIJKLMOJPQRSTUVWYXYZabcdefghijklmnopqrstuvwxyz".chars() {
            let escaped = c.escape_unicode();
            for out_c in escaped.skip(3).take_while(|x| *x != '}') {
                println!("{}", out_c);
            }
        }
    }
}
