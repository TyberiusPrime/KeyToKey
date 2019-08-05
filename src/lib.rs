//todo:
// shift remaps on layers (ie. disassociate the premade shift-combos)
// combos
// tapdance enhancemeants, on_each_tap, and max_taps?
// toggle on x presses? - should be a tapdance impl?

// premade toggle/oneshot modifiers
// key lock (repeat next key until it is pressed again)
// mouse keys? - probably out of scope of this libary
// steganograpyh
// unsupported: disabling a layer when one of it's rewriteTo are active?

#![allow(dead_code)]
#![feature(drain_filter)]
#![no_std]

pub mod debug_handlers;
pub mod handlers;
mod key_codes;
mod key_stream;
mod matrix;
mod test_helpers;

extern crate alloc;
extern crate no_std_compat;

pub use crate::handlers::ProcessKeys;
use crate::key_codes::UNICODE_BELOW_256;
pub use crate::key_codes::{AcceptsKeycode, KeyCode};
use crate::key_stream::Key;
pub use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use core::convert::TryInto;
use no_std_compat::prelude::v1::*;

/// current keyboard state.
#[derive(Debug)]
pub struct KeyboardState {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
    pub unicode_mode: UnicodeSendMode,
    enabled_handlers: Vec<bool>,
}

impl KeyboardState {
    pub fn new() -> KeyboardState {
        KeyboardState {
            shift: false,
            ctrl: false,
            alt: false,
            meta: false,
            unicode_mode: UnicodeSendMode::Linux,
            enabled_handlers: Vec::new(),
        }
    }
    pub fn enable_handler(&mut self, no: HandlerID) {
        self.enabled_handlers[no] = true;
    }

    pub fn disable_handler(&mut self, no: HandlerID) {
        self.enabled_handlers[no] = false;
    }

    pub fn toggle_handler(&mut self, no: HandlerID) {
        self.enabled_handlers[no] = !self.enabled_handlers[no];
    }

    pub fn is_handler_enabled(&self, no: HandlerID) -> bool {
        self.enabled_handlers[no]
    }
}

/// the main keyboard struct
///
/// add handlers wit add_handler,
/// then pass it to matrix.MatrixToStream.update()
/// to start processing keys.
pub struct Keyboard<'a, T: USBKeyOut> {
    events: Vec<(Event, EventStatus)>,
    running_number: u8,
    handlers: Vec<Box<dyn ProcessKeys<T> + Send + 'a>>,
    pub output: T,
}

type HandlerID = usize;

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
    /// which you may use for enable_handler/disable_handler
    ///
    /// by default, most handlers start in the enabled state.
    pub fn add_handler(&mut self, handler: Box<dyn ProcessKeys<T> + Send + 'a>) -> HandlerID {
        self.output
            .state()
            .enabled_handlers
            .push(handler.default_enabled());
        self.handlers.push(handler);
        return self.handlers.len() - 1;
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
        let enabled = self.output.state().enabled_handlers.clone();
        for (h, e) in self.handlers.iter_mut().zip(enabled.iter()) {
            if *e {
                h.process_keys(&mut self.events, &mut self.output);
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
#[derive(Clone, Copy, Debug)]
pub enum UnicodeSendMode {
    //default X
    Linux = 1,
    /// use https://github.com/samhocevar/wincompose
    WinCompose,
    // used by the tests
    Debug,
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

    fn send_unicode(&mut self, c: char) {
        match self.state().unicode_mode {
            UnicodeSendMode::Linux => {
                self.send_keys(&[KeyCode::LCtrl, KeyCode::LShift, KeyCode::U]);
                let escaped = c.escape_unicode();
                for out_c in escaped.skip(3).take_while(|x| *x != '}') {
                    self.send_keys(&[KeyCode::LCtrl, KeyCode::LShift, hex_digit_to_keycode(out_c)]);
                }
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
            UnicodeSendMode::Debug => {
                let mut buf = [0, 0, 0, 0];
                c.encode_utf8(&mut buf);
                self.send_keys(&[((buf[0] as u32) + UNICODE_BELOW_256).try_into().unwrap()]);
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

#[cfg(test)]
extern crate parking_lot;
