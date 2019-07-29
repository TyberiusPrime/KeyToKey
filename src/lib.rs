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

mod debug_handlers;
mod handlers;
mod key_codes;
mod key_stream;
mod matrix;
mod test_helpers;

extern crate alloc;
extern crate no_std_compat;

pub use crate::handlers::*;
use crate::key_codes::{AcceptsKeycode, KeyCode, UNICODE_BELOW_256};
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus, Key};
use core::convert::TryInto;
use no_std_compat::prelude::v1::*;

/// current keyboard state.
#[derive(Debug)]
pub struct KeyboardState {
    shift: bool,
    ctrl: bool,
    alt: bool,
    meta: bool,
    unicode_mode: UnicodeSendMode,
}

impl KeyboardState {
    fn new() -> KeyboardState {
        KeyboardState {
            shift: false,
            ctrl: false,
            alt: false,
            meta: false,
            unicode_mode: UnicodeSendMode::Linux,
        }
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
    handlers: Vec<Box<dyn ProcessKeys<T> + 'a>>,
    enabled: Vec<bool>,
    output: T,
}

type HandlerID = usize;

impl<'a, T: USBKeyOut> Keyboard<'a, T> {
    fn new(output: T) -> Keyboard<'a, T> {
        Keyboard {
            events: Vec::new(),
            running_number: 0,
            handlers: Vec::new(),
            enabled: Vec::new(), //possibly replace by fancy bit addresing variant?
            output,
        }
    }

    /// add a handler, return a HandlerID
    /// which you may use for enable_handler/disable_handler
    ///
    /// by default, most handlers start in the enabled state.
    pub fn add_handler(&mut self, handler: Box<dyn ProcessKeys<T> + 'a>) -> HandlerID {
        self.handlers.push(handler);
        self.enabled.push(true);
        return self.handlers.len() - 1;
    }

    pub fn enable_handler(&mut self, no: HandlerID) {
        self.enabled[no] = true;
    }

    pub fn disable_handler(&mut self, no: HandlerID) {
        self.enabled[no] = false;
    }

    /// handle an update to the event stream
    ///
    /// This returns OK(()) if all keys are handled by the handlers
    /// and an Err(()) otherwise.
    /// that way the down stream can decide what to do
    /// (tests: panic. Firmare/MatrixToStream -> drop unhandled events)
    fn handle_keys(&mut self) -> Result<(), ()> {
        for (_e, status) in self.events.iter_mut() {
            *status = EventStatus::Unhandled;
        }
        for (h, e) in self.handlers.iter_mut().zip(self.enabled.iter()) {
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
    fn clear_unhandled(&mut self) {
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
                self.send_empty();
            }
            UnicodeSendMode::Debug => {
                let mut buf = [0, 0, 0, 0];
                c.encode_utf8(&mut buf);
                self.send_keys(&[(
                    (buf[0] as u32) + UNICODE_BELOW_256)
                    .try_into().unwrap()]);
            }
        }
    }

    /// send a utf-8 string to the host
    /// all characters are converted into unicode input!
    fn send_string(&mut self, s: &str) {
        for c in s.chars() {
            self.send_unicode(c);
            // option: send simple ones directly?
            /*
            if 'a' <= c && c <= 'z' {
                let mut ascii = [0 as u8];
                c.encode_utf8(&mut ascii);
                let keycode: u8 = KeyCode::A.into();
                let keycode = keycode + (ascii[0] - 97);
                let keycode: KeyCode = keycode.try_into().unwrap();
                self.send_keys(&[keycode]);
                self.send_keys(&[]); // send release
            }
            */
        }
    }
}

//so the tests 'just work'.
#[cfg(test)]
#[macro_use]
extern crate std;
