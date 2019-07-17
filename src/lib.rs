#![allow(dead_code)]
#![feature(drain_filter)]
#![no_std]

mod key_codes;

extern crate alloc;
extern crate no_std_compat;

use crate::key_codes::KeyCode;
use core::cell::RefCell;
use core::convert::TryInto;
use no_std_compat::prelude::v1::*;

pub const UNICODE_BELOW_256: u32 = 0x100000;

#[derive(PartialEq, Debug)]
struct Key {
    keycode: u32,
    ms_since_last: u16,
    running_number: u8,
    flag: u8,
}

impl Key {
    fn new(keycode: u32) -> Key {
        Key {
            keycode,
            ms_since_last: 0,
            running_number: 0,
            flag: 0,
        }
    }
}

#[derive(PartialEq, Debug)]
enum Event {
    KeyPress(Key),
    KeyRelease(Key),
    TimeOut(u16),
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum EventStatus {
    Unhandled,
    Handled,
    Ignored,
}

fn iter_unhandled_mut(
    events: &mut Vec<(Event, EventStatus)>,
) -> impl DoubleEndedIterator<Item = &mut (Event, EventStatus)> {
    events
        .iter_mut()
        .filter(|(_e, status)| EventStatus::Unhandled == *status)
}

trait AcceptsKeycode {
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

struct KeyboardState {
    shift: bool,
    ctrl: bool,
    alt: bool,
    meta: bool,
 
}

struct Keyboard<'a, T: USBKeyOut> {
    events: Vec<(Event, EventStatus)>,
    running_number: u8,
    handlers: Vec<Box<dyn ProcessKeys<T> + 'a>>,
    output: T,
    state: KeyboardState,
   //unicode_sendmode: UnicodeSendMode,

}

impl<'a, T: USBKeyOut> Keyboard<'_, T> {
    fn new(handlers: Vec<Box<dyn ProcessKeys<T> + 'a>>, output: T) -> Keyboard<T> {
        Keyboard {
            events: Vec::new(),
            running_number: 0,
            handlers,
            output,
            state: KeyboardState {
                shift: false,
                ctrl: false,
                alt: false,
                meta: false,
            },
        }
    }

    fn handle_keys(&mut self) -> Result<(), String> {
        for (_e, status) in self.events.iter_mut() {
            *status = EventStatus::Unhandled;
        }
        for h in self.handlers.iter_mut() {
            h.process_keys(&mut self.events, &mut self.output, &mut self.state);
        }
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
            return Err(format!("Unhandled input! {:?}", self.events));
        }
        Ok(())
    }

    fn add_keypress<X: AcceptsKeycode>(&mut self, keycode: X, ms_since_last: u16) {
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
    fn add_keyrelease<X: AcceptsKeycode>(&mut self, keycode: X, ms_since_last: u16) {
        let e = Key {
            keycode: keycode.to_u32(),
            ms_since_last,
            running_number: self.running_number,
            flag: 0
        };
        self.running_number += 1;
        self.events
            .push((Event::KeyRelease(e), EventStatus::Unhandled));
    }

    fn add_timeout(&mut self, ms_since_last: u16) {
        if let Some((event, _status)) = self.events.iter().last() {
            if let Event::TimeOut(_) = event {
                self.events.pop();
            }
        }
        self.events
            .push((Event::TimeOut(ms_since_last), EventStatus::Unhandled));
    }
}

#[derive(Clone, Copy)]
enum UnicodeSendMode {
    Linux = 1,
    WinCompose,
    Debug,
}

trait ProcessKeys<T: USBKeyOut> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T,
    state: &mut KeyboardState) -> ();
    fn enable(&self) {}
    fn disable(&self) {}
}

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

trait USBKeyOut {
    fn send_keys(&mut self, keys: &[KeyCode]);
    fn register_key(&mut self, key: KeyCode);
    fn send_registered(&mut self);
    fn send_empty(&mut self);

    fn get_unicode_mode(&self) -> UnicodeSendMode;
    fn set_unicode_mode(&mut self, mode: UnicodeSendMode);
    fn send_unicode(&mut self, c: char) {
        match self.get_unicode_mode() {
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
                self.send_keys(&[buf[0].try_into().unwrap()]);
            }
        }
    }
    fn send_string(&mut self, s: &str) {
        for c in s.chars() {
            self.send_unicode(c);
            // option: forget about all this.
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

/// The default bottom layer
///
/// this simulates a bog standard regular USB
/// Keyboard.
/// Just map your keys to the usb keycodes.
///
/// key repeat is whatever usb does...
struct USBKeyboard {
    }

impl USBKeyboard {
    fn new() -> USBKeyboard {
        USBKeyboard {
        }
    }
}

impl<T: USBKeyOut> ProcessKeys<T> for USBKeyboard {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T,
    state: &mut KeyboardState) -> () {
        //step 0: on key release, remove all prior key presses.
        let mut codes_to_delete: Vec<u32> = Vec::new();
        for (e, status) in iter_unhandled_mut(events).rev() {
            //note that we're doing this in reverse, ie. releases happen before presses.
            match e {
                Event::KeyRelease(kc) => {
                    if kc.keycode < 256 {
                        if !codes_to_delete.contains(&kc.keycode) {
                            codes_to_delete.push(kc.keycode);
                        }
                        *status = EventStatus::Handled;
                    }
                }
                Event::KeyPress(kc) => {
                    let mut send = false;
                    if codes_to_delete.contains(&kc.keycode) {
                        *status = EventStatus::Handled;
                        if kc.flag == 0 {
                            //we have never send this before
                            send = true;
                        }
                    } else {
                        send = true;
                    }
                    if kc.keycode < 256 {
                        let oc: Result<KeyCode, String> = (kc.keycode as u8).try_into();
                        match oc {
                            Ok(x) => {
                                if send {
                                    output.register_key(x);
                                }
                                if *status != EventStatus::Handled {
                                    *status = EventStatus::Ignored; //so we may resend it...
                                }
                            }
                            Err(_) => *status = EventStatus::Handled, //throw it away, will ya?
                        };
                        kc.flag = 1;
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
        //dbg!(&result);
        if state.shift {
            output.register_key(KeyCode::LShift);
        }
        if state.ctrl {
            output.register_key(KeyCode::LCtrl);
        }
        if state.alt {
            output.register_key(KeyCode::LAlt);
        }
        if state.meta {
            output.register_key(KeyCode::LGui);
        }




        output.send_registered();
    }
}

/// This processor sends unicode 'characters'
/// just map your keys to unicode 'characters' (if code > 256)
/// for unicode codes < 256 use unicode code + UNICODE_BELOW_256
/// (=0x100000).
/// sending happens on keyrelease - no key repeat
///
/// use the range 0xF0000..=0xFFFFD, or 0x1000FF..=0x10FFFD
///  for custom key codes that are note processed
struct UnicodeKeyboard {}
impl UnicodeKeyboard {
    fn is_unicode_keycode(keycode: u32) -> bool {
        match keycode {
            0..=0xFF => false,            //these we ignore
            0xF0000..=0xFFFFD => false,   //unicode private character range A
            0x1000FF..=0x10FFFD => false, //unicode private character range b (minus those we use for codes < 256)
            _ => true,
        }
    }
    fn keycode_to_unicode(keycode: u32) -> u32 {
        if keycode < 0x100000 {
            keycode
        } else {
            keycode - 0x100000
        }
    }
}
impl<T: USBKeyOut> ProcessKeys<T> for UnicodeKeyboard {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T,
    _state: &mut KeyboardState) -> () {
        for (event, status) in iter_unhandled_mut(events) {
            match event {
                Event::KeyPress(kc) => {
                    if UnicodeKeyboard::is_unicode_keycode(kc.keycode) {
                        *status = EventStatus::Handled;
                    }
                }
                Event::KeyRelease(kc) => {
                    if UnicodeKeyboard::is_unicode_keycode(kc.keycode) {
                        let c = no_std_compat::char::from_u32(UnicodeKeyboard::keycode_to_unicode(
                            kc.keycode,
                        ));
                        if let Some(c) = c {
                            output.send_unicode(c);
                        }
                        *status = EventStatus::Handled;
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
    }
}

#[derive(PartialEq)]
enum MatchResult<'a> {
    Match(&'a str),
    WontMatch,
    NeedsMoreInput,
}

struct Leader<'a> {
    trigger: u32,
    mappings: Vec<(Vec<u32>, &'a str)>,
    failure: &'a str,
    prefix: Vec<u32>, //todo: refactor to not need this but use repeated iterators?
    active: bool,
}

impl<'a> Leader<'a> {
    fn new<T: AcceptsKeycode>(
        trigger: impl AcceptsKeycode,
        mappings: Vec<(Vec<T>, &'a str)>,
        failure: &'a str,
    ) -> Leader<'a> {
        //Todo: Figure out how to check for mappings that are prefixes of other mappings
        //(and therefore impossible) at compile time
        Leader {
            trigger: trigger.to_u32(),
            mappings: mappings
                .into_iter()
                .map(|(a, b)| (a.into_iter().map(|x| x.to_u32()).collect(), b))
                .collect(),
            failure,
            prefix: Vec::new(),
            active: false,
        }
    }
    fn match_prefix(&self) -> MatchResult {
        let mut result = MatchResult::WontMatch;
        for (seq, out) in self.mappings.iter() {
            if seq.len() < self.prefix.len() {
                continue;
            }
            if self.prefix.iter().zip(seq.iter()).all(|(a, b)| a == b) {
                if seq.len() == self.prefix.len() {
                    return MatchResult::Match(out);
                } else {
                    result = MatchResult::NeedsMoreInput;
                }
            }
        }
        result
    }
}

impl<T: USBKeyOut> ProcessKeys<T> for Leader<'_> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T,
    _state: &mut KeyboardState) -> () {
        for (event, status) in iter_unhandled_mut(events) {
            match event {
                Event::KeyRelease(kc) => {
                    if self.active {
                        self.prefix.push(kc.keycode);
                        match self.match_prefix() {
                            MatchResult::Match(s) => {
                                output.send_string(s);
                                self.active = false;
                                self.prefix.clear()
                            }
                            MatchResult::WontMatch => {
                                output.send_string(self.failure);
                                self.active = false;
                                self.prefix.clear()
                            }
                            MatchResult::NeedsMoreInput => {}
                        }
                        *status = EventStatus::Handled;
                    } else if kc.keycode == self.trigger {
                        if !self.active {
                            self.active = true;
                        }
                        *status = EventStatus::Handled;
                    }
                }
                Event::KeyPress(kc) => {
                    if kc.keycode == self.trigger {
                        *status = EventStatus::Handled;
                    } else if self.active {
                        // while active, we eat all KeyPresses and only parse KeyRelease
                        *status = EventStatus::Handled;
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
    }
}

enum LayerAction<'a> {
    RewriteTo(u32),
    SendString(&'a str),
    //    Callback(fn(&mut T) -> (), fn(&mut T) -> ()),
}
struct Layer<'a> {
    rewrites: Vec<(u32, LayerAction<'a>)>,
    enabled: RefCell<bool>,
}

impl Layer<'_> {
    fn new<F: AcceptsKeycode>(rewrites: Vec<(F, LayerAction)>) -> Layer<'_> {
        Layer {
            rewrites: rewrites
                .into_iter()
                .map(|(trigger, action)| (trigger.to_u32(), action))
                .collect(),
            enabled: RefCell::new(true),
        }
    }
}

impl<T: USBKeyOut> ProcessKeys<T> for Layer<'_> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T,
    _state: &mut KeyboardState) -> () {
        if !*self.enabled.borrow() {
            return;
        };
        for (event, status) in events.iter_mut() {
            match event {
                Event::KeyRelease(kc) => {
                    for (from, to) in self.rewrites.iter() {
                        if *from == kc.keycode {
                            match to {
                                LayerAction::RewriteTo(to_keycode) => {
                                    kc.keycode = *to_keycode;
                                }
                                LayerAction::SendString(s) => {
                                    output.send_string(s);
                                    *status = EventStatus::Handled;
                                }
                            }
                        }
                    }
                }
                Event::KeyPress(kc) => {
                    for (from, to) in self.rewrites.iter() {
                        if *from == kc.keycode {
                            match to {
                                LayerAction::RewriteTo(to_keycode) => {
                                    kc.keycode = *to_keycode;
                                }
                                _ => *status = EventStatus::Handled,
                            }
                        }
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
    }
    fn enable(&self) {
        *self.enabled.borrow_mut() = true;
    }
    fn disable(&self) {
        *self.enabled.borrow_mut() = false;
    }
}

/// The simples callback -
/// call on_press(output: impl USBKeyOut) on key press
/// and on_release(output) on release))
/// trigger may be any keycode,
/// but preferentialy from the region 0xF00FF..=0xFFFFD
/// which is not used by either UnicodeKeyboard or UsbKeyboard
struct PressReleaseMacro<'a, T, F1, F2> {
    keycode: u32,
    on_press: Option<F1>,
    on_release: Option<F2>,
    phantom: core::marker::PhantomData<&'a T>,
}
impl<'a, T: USBKeyOut, F1: FnMut(&mut T), F2: FnMut(&mut T)> PressReleaseMacro<'a, T, F1, F2> {
    fn new(
        trigger: impl AcceptsKeycode,
        on_press: Option<F1>,
        on_release: Option<F2>,
    ) -> PressReleaseMacro<'a, T, F1, F2> {
        PressReleaseMacro {
            keycode: trigger.to_u32(),
            on_press,
            on_release,
            phantom: core::marker::PhantomData,
        }
    }
}

impl<T: USBKeyOut, F1: FnMut(&mut T), F2: FnMut(&mut T)> ProcessKeys<T>
    for PressReleaseMacro<'_, T, F1, F2>
{
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T,
    _state: &mut KeyboardState) -> () {
        for (event, status) in events.iter_mut() {
            match event {
                Event::KeyPress(kc) => {
                    if kc.keycode == self.keycode {
                        *status = EventStatus::Handled;
                        match &mut self.on_press {
                            Some(x) => (*x)(output),
                            None => {}
                        }
                    }
                }
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.keycode {
                        *status = EventStatus::Handled;
                        match &mut self.on_release {
                            Some(x) => (*x)(output),
                            None => {}
                        }
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
    }
}

/// a macro that is called 'on' on the the first keypress
/// and off on the second keyrelease.
/// Using this you can implement e.g. sticky modifiers
///
struct StickyMacro<'a, T, F1: FnMut(&mut T), F2: FnMut(&mut T)> {
    keycode: u32,
    on_toggle_on: F1,
    on_toggle_off: F2,
    active: u8,
    phantom: core::marker::PhantomData<&'a T>,
}
impl<'a, T: USBKeyOut, F1: FnMut(&mut T), F2: FnMut(&mut T)> StickyMacro<'a, T, F1, F2> {
    fn new(
        trigger: impl AcceptsKeycode,
        on_toggle_on: F1,
        on_toggle_off: F2,
    ) -> StickyMacro<'a, T, F1, F2> {
        StickyMacro {
            keycode: trigger.to_u32(),
            on_toggle_on,
            on_toggle_off,
            active: 0,
            phantom: core::marker::PhantomData,
        }
    }
}

impl<T: USBKeyOut, F1: FnMut(&mut T), F2: FnMut(&mut T)> ProcessKeys<T>
    for StickyMacro<'_, T, F1, F2>
{
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T,
    _state: &mut KeyboardState) -> () {
        for (event, status) in events.iter_mut() {
            //a sticky key
            // on press if not active -> active
            // on 2nd release if active -> deactivate
            match event {
                Event::KeyPress(kc) => {
                    if kc.keycode == self.keycode {
                        if self.active == 0 {
                            self.active = 1;
                            (self.on_toggle_on)(output);
                        } else {
                            self.active = 2;
                        }
                        *status = EventStatus::Handled;
                    }
                }
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.keycode {
                        if self.active == 2 {
                            (self.on_toggle_off)(output);
                        }
                        *status = EventStatus::Handled;
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
    }
}

/// A OneShot key
/// press it, on_toggle_on will be called,
/// on_toggle_off will be called after the next key
/// release of if the OneShot trigger is pressed again
struct OneShot<'a, T, F1: FnMut(&mut T), F2: FnMut(&mut T)> {
    keycode: u32,
    on_toggle_on: F1,
    on_toggle_off: F2,
    active: bool,
    phantom: core::marker::PhantomData<&'a T>,
}
impl<'a, T: USBKeyOut, F1: FnMut(&mut T), F2: FnMut(&mut T)> OneShot<'a, T, F1, F2> {
    fn new(
        trigger: impl AcceptsKeycode,
        on_toggle_on: F1,
        on_toggle_off: F2,
    ) -> OneShot<'a, T, F1, F2> {
        OneShot {
            keycode: trigger.to_u32(),
            on_toggle_on,
            on_toggle_off,
            active: false,
            phantom: core::marker::PhantomData,
        }
    }
}

impl<T: USBKeyOut, F1: FnMut(&mut T), F2: FnMut(&mut T)> ProcessKeys<T> for OneShot<'_, T, F1, F2> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T,
    _state: &mut KeyboardState) -> () {
        for (event, status) in events.iter_mut() {
            //a sticky key
            // on press if not active -> active
            // on other key release -> deactivate
            // on press if active -> deactive
            // on release -> noop?
            match event {
                Event::KeyPress(kc) => {
                    if kc.keycode == self.keycode {
                        *status = EventStatus::Handled;
                        if self.active {
                            self.active = false;
                            (self.on_toggle_off)(output);
                        } else {
                            self.active = true;
                            (self.on_toggle_on)(output);
                        }
                    }
                }
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.keycode {
                        *status = EventStatus::Handled;
                    } else {
                        self.active = false;
                        (self.on_toggle_off)(output);
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
    }
}

struct TapDance<'a, T, F> {
    trigger: u32,
    tap_count: u8,
    on_tap_complete: F, //todo: should we differentiate between timeout and other key by passing an enum?
    //todo: add on_each_tap...
    timeout_ms: u16,
    phantom: core::marker::PhantomData<&'a T>,
}

impl<'a, T: USBKeyOut, F: FnMut(u8, &mut T)> TapDance<'a, T, F> {
    fn new(trigger: impl AcceptsKeycode, on_tap_complete: F) -> TapDance<'a, T, F> {
        TapDance {
            trigger: trigger.to_u32(),
            tap_count: 0,
            on_tap_complete,
            timeout_ms: 250,
            phantom: core::marker::PhantomData,
        }
    }
}

impl<T: USBKeyOut, F: FnMut(u8, &mut T)> ProcessKeys<T> for TapDance<'_, T, F> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T,
    _state: &mut KeyboardState) -> () {
        for (event, status) in iter_unhandled_mut(events) {
            match event {
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.trigger {
                        *status = EventStatus::Handled;
                    }
                }
                Event::KeyPress(kc) => {
                    if kc.keycode != self.trigger {
                        if self.tap_count > 0 {
                            (self.on_tap_complete)(self.tap_count, output);
                            self.tap_count = 0;
                        }
                    } else {
                        self.tap_count += 1;
                        *status = EventStatus::Handled;
                    }
                }
                Event::TimeOut(ms_since_last) => {
                    if self.tap_count > 0 && *ms_since_last > self.timeout_ms {
                        (self.on_tap_complete)(self.tap_count, output);
                        self.tap_count = 0;
                    }
                }
            }
        }
    }
}

struct SpaceCadet<'a, T, F1, F2> {
    trigger: u32,
    on_activate: F1,
    on_deactivate: F2,
    press_number: u8,
    down: bool,
    activated: bool,
    phantom: core::marker::PhantomData<&'a T>,
}

impl<'a, T: USBKeyOut, F1: FnMut(&mut T), F2: FnMut(&mut T)> SpaceCadet<'a, T, F1, F2> {
    fn new(
        trigger: impl AcceptsKeycode,
        on_activate: F1,
        on_deactivate: F2,
    ) -> SpaceCadet<'a, T, F1, F2> {
        SpaceCadet {
            trigger: trigger.to_u32(),
            on_activate,
            on_deactivate,
            press_number: 0,
            down: false,
            activated: false,
            phantom: core::marker::PhantomData,
        }
    }
}

impl<T: USBKeyOut, F1: FnMut(&mut T), F2: FnMut(&mut T)> ProcessKeys<T>
    for SpaceCadet<'_, T, F1, F2>
{
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T,
    _state: &mut KeyboardState) -> () {
        let mut initial_keypress_status: Option<EventStatus> = None;
        for (event, status) in iter_unhandled_mut(events) {
            match event {
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.trigger {
                        self.down = false;
                        if kc.running_number == self.press_number + 1 {
                            // a tap
                            //let the downstream handle it!
                            initial_keypress_status = Some(EventStatus::Unhandled);
                        } else {
                            (self.on_deactivate)(output);
                            *status = EventStatus::Handled;
                            initial_keypress_status = Some(EventStatus::Handled);
                        }
                    }
                }
                Event::KeyPress(kc) => {
                    if kc.keycode == self.trigger {
                        *status = EventStatus::Ignored; //skip the scan this time
                        self.press_number = kc.running_number;
                        self.down = true
                    } else if self.down {
                        //trigger has been seen..
                        if !self.activated {
                            (self.on_activate)(output);
                        }
                        self.activated = true;
                        initial_keypress_status = Some(EventStatus::Ignored);
                        //remeber, this is a non-related keypress.
                        //*status = EventStatus::Handled;
                    }
                }
                Event::TimeOut(_) => {}
            }
        }

        match initial_keypress_status {
            Some(new_status) => {
                for (event, status) in events.iter_mut() {
                    match event {
                        Event::KeyPress(kc) => {
                            if kc.keycode == self.trigger {
                                *status = new_status;
                            }
                        }
                        _ => {}
                    }
                }
            }
            None => {}
        }
    }
}

struct AutoShift {
    shift_letters: bool,
    shift_numbers: bool,
    shift_special: bool,
    threshold_ms: u16,
}

impl AutoShift {
    fn new(threshold_ms: u16) -> AutoShift {
        AutoShift {
            shift_letters: true,
            shift_numbers: true,
            shift_special: true,
            threshold_ms,
        }
    }

    fn should_autoshift(&self, keycode: u32) -> bool {
        return (self.shift_letters
            && keycode >= KeyCode::A.to_u32()
            && keycode <= KeyCode::Z.to_u32())
            | (self.shift_numbers
                && keycode >= KeyCode::Kb1.to_u32()
                && keycode <= KeyCode::Kb0.to_u32())
            | (self.shift_special
                && keycode >= KeyCode::Minus.to_u32()
                && keycode <= KeyCode::Slash.to_u32());
    }
}

impl<T: USBKeyOut> ProcessKeys<T> for AutoShift {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T,
    _state: &mut KeyboardState) -> () {
        let mut presses = Vec::new();
        let mut handled = Vec::new();
        for (event, status) in iter_unhandled_mut(events) {
            match event {
                Event::KeyPress(kc) => {
                    if self.should_autoshift(kc.keycode) {
                        *status = EventStatus::Ignored;
                        presses.push((kc.keycode, kc.ms_since_last));
                    }
                }
                Event::KeyRelease(kc) => {
                    if self.should_autoshift(kc.keycode) {
                        for (other_keycode, timestamp) in presses.iter() {
                            if *other_keycode == kc.keycode {
                                let delta = kc.ms_since_last - timestamp;
                                if delta >= self.threshold_ms {
                                    output.send_keys(&[
                                        KeyCode::LShift,
                                        (kc.keycode as u8).try_into().unwrap(),
                                    ])
                                }
                                else {
                                    output.send_keys(&[
                                        (kc.keycode as u8).try_into().unwrap(),
                                    ])

                                }
                                handled.push(kc.keycode)
                            }
                        }
                        *status = EventStatus::Handled;
                    }
                }
                _ => {}
            }
        }
        if !handled.is_empty() {
            for (event, status) in events.iter_mut() {
                match event {
                    Event::KeyPress(kc) => {
                        if handled.contains(&kc.keycode) {
                            *status = EventStatus::Handled;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

//todo:

//lower priority
// combos
// auto shift
// toggle on x presses?
// key lock (repeat next key until it is pressed again)
// mouse keys? - probably out of scope of this libary
// steganograpyh

#[cfg(test)]
#[macro_use]
extern crate std;
mod tests {
    use crate::key_codes::KeyCode;

    #[allow(unused_imports)]
    use crate::{
        Event, Keyboard, Layer, LayerAction, Leader, MatchResult, OneShot, PressReleaseMacro,
        ProcessKeys, SpaceCadet, StickyMacro, TapDance, USBKeyOut, USBKeyboard, UnicodeKeyboard, AutoShift,
        UnicodeSendMode, UNICODE_BELOW_256,
    };
    use no_std_compat::prelude::v1::*;

    #[allow(unused_imports)]
    use no_std_compat::cell::RefCell;
    #[allow(unused_imports)]
    use no_std_compat::rc::Rc;

    struct KeyOutCatcher {
        keys_registered: Vec<u8>,
        reports: Vec<Vec<u8>>,
        unicode_mode: UnicodeSendMode,
    }
    impl KeyOutCatcher {
        fn new() -> KeyOutCatcher {
            KeyOutCatcher {
                keys_registered: Vec::new(),
                reports: Vec::new(),
                unicode_mode: UnicodeSendMode::Linux,
            }
        }
        // for testing, clear the catcher of everything
        fn clear(&mut self) {
            self.keys_registered.clear();
            self.reports.clear();
        }
    }
    impl USBKeyOut for KeyOutCatcher {
        fn get_unicode_mode(&self) -> UnicodeSendMode {
            return self.unicode_mode;
        }
        fn set_unicode_mode(&mut self, mode: UnicodeSendMode) {
            self.unicode_mode = mode;
        }

        fn send_keys(&mut self, keys: &[KeyCode]) {
            self.reports
                .push(keys.into_iter().map(|&x| x.into()).collect());
        }
        fn register_key(&mut self, key: KeyCode) {
            if !self.keys_registered.iter().any(|x| *x == key.into()) {
                self.keys_registered.push(key.into());
            }
        }
        fn send_registered(&mut self) {
            self.reports.push(self.keys_registered.clone());
            self.keys_registered.clear();
        }
        fn send_empty(&mut self) {
            self.reports.push(Vec::new());
        }
    }
    fn check_output(keyboard: &Keyboard<KeyOutCatcher>, should: &[&[KeyCode]]) {
        assert!(should.len() == keyboard.output.reports.len());
        for (ii, report) in should.iter().enumerate() {
            assert!(keyboard.output.reports[ii].len() == report.len());
            for k in report.iter() {
                let kcu: u8 = (*k).into();
                assert!(keyboard.output.reports[ii].contains(&kcu));
            }
        }
    }
    #[test]
    fn test_usbkeyboard_single_key() {
        let h = vec![Box::new(USBKeyboard::new()) as Box<dyn ProcessKeys<KeyOutCatcher>>];
        let mut keyboard = Keyboard::new(h, KeyOutCatcher::new());
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::A]]);
        assert!(!keyboard.events.is_empty());
        keyboard.add_keyrelease(KeyCode::A, 20);
        assert!(keyboard.events.len() == 2);
        keyboard.output.clear();
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        assert!(keyboard.events.is_empty());
    }
    #[test]
    fn test_usbkeyboard_multiple_key() {
        use KeyCode::*;
        let h = vec![Box::new(USBKeyboard::new()) as Box<dyn ProcessKeys<KeyOutCatcher>>];
        let mut keyboard = Keyboard::new(h, KeyOutCatcher::new());
        keyboard.add_keypress(A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[A]]);
        assert!(!keyboard.events.is_empty());

        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[A, X]]);
        assert!(!keyboard.events.is_empty());

        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::A, 20);
        assert!(keyboard.events.len() == 3);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[X]]);
        assert!(!keyboard.events.is_empty());

        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::X, 20);
        assert!(keyboard.events.len() == 2);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        assert!(keyboard.events.is_empty());
    }

    #[test]
    fn test_unicode_keyboard_linux() {
        use KeyCode::*;
        let ub = UnicodeKeyboard {};
        let h = vec![Box::new(ub) as Box<dyn ProcessKeys<KeyOutCatcher>>];
        let mut keyboard = Keyboard::new(h, KeyOutCatcher::new());
        keyboard.output.set_unicode_mode(UnicodeSendMode::Linux);
        //no output on press
        keyboard.add_keypress(0x00E4u32 + UNICODE_BELOW_256, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.reports.len() == 0);
        assert!(keyboard.events.is_empty()); // we eat the keypress though

        keyboard.add_keyrelease(0x00E4 + UNICODE_BELOW_256, 0);
        keyboard.handle_keys().unwrap();
        check_output(
            &keyboard,
            &[
                &[U, LShift, LCtrl],
                &[E, LShift, LCtrl],
                &[Kb4, LShift, LCtrl],
                &[],
            ],
        );

        assert!(keyboard.events.is_empty()); // we eat the keypress though
    }

    #[test]
    fn test_unicode_keyboard_wincompose() {
        use KeyCode::*;
        let ub = UnicodeKeyboard {};
        let h = vec![Box::new(ub) as Box<dyn ProcessKeys<KeyOutCatcher>>];
        let mut keyboard = Keyboard::new(h, KeyOutCatcher::new());
        keyboard.output.set_unicode_mode(UnicodeSendMode::WinCompose);
        //no output on press
        keyboard.add_keypress(0x03B4u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.reports.len() == 0);
        assert!(keyboard.events.is_empty()); // we eat the keypress though

        keyboard.add_keyrelease(0x03B4, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[RAlt], &[U], &[Kb3], &[B], &[Kb4], &[]]);
        assert!(keyboard.events.is_empty()); // we eat the keypress though
    }

    #[test]
    fn test_unicode_while_depressed() {
        use KeyCode::*;
        let h = vec![
            Box::new(UnicodeKeyboard {}) as Box<dyn ProcessKeys<KeyOutCatcher>>,
            Box::new(USBKeyboard::new()),
        ];
        let mut keyboard = Keyboard::new(h, KeyOutCatcher::new());
        keyboard.output.set_unicode_mode(UnicodeSendMode::WinCompose);
        keyboard.add_keypress(A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[A]]);
        keyboard.output.clear();
        keyboard.add_keypress(0x3B4u32, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[A]]);
        keyboard.add_keyrelease(0x3B4, 0);
        keyboard.output.clear();
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[RAlt], &[U], &[Kb3], &[B], &[Kb4], &[], &[A]]);
        keyboard.add_keyrelease(A, 0);
        keyboard.output.clear();
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        assert!(keyboard.events.is_empty());
    }

    #[test]
    fn test_panic_on_unhandled() {
        let h = vec![Box::new(USBKeyboard::new()) as Box<dyn ProcessKeys<KeyOutCatcher>>];
        let mut keyboard = Keyboard::new(h, KeyOutCatcher::new());
        keyboard.add_keypress(0xF0000u32, 0);
        assert!(keyboard.handle_keys().is_err());
    }

    #[test]
    fn test_toggle_macro() {
        let down_counter = RefCell::new(0);
        let up_counter = RefCell::new(0);
        let t = OneShot::new(
            0xF0000u32,
            |output: &mut KeyOutCatcher| {
                output.send_keys(&[KeyCode::H]);
                let mut dc = down_counter.borrow_mut();
                *dc += 1;
            },
            |_output| {
                let mut dc = up_counter.borrow_mut();
                *dc += 1;
            },
        );
        let h = vec![
            Box::new(t) as Box<dyn ProcessKeys<KeyOutCatcher>>,
            Box::new(USBKeyboard::new()),
        ];

        let mut keyboard = Keyboard::new(h, KeyOutCatcher::new());
        //first press - sets
        keyboard.add_keypress(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 0);
        assert!(keyboard.events.is_empty());
        check_output(&keyboard, &[&[KeyCode::H], &[]]);
        keyboard.output.clear();

        //first release - no change
        keyboard.add_keyrelease(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 0);
        assert!(keyboard.events.is_empty());

        //second press - unsets
        keyboard.add_keypress(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 1);
        assert!(keyboard.events.is_empty());

        //second release - no change
        keyboard.add_keyrelease(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 1);
        assert!(keyboard.events.is_empty());

        //third press - sets
        keyboard.add_keypress(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 2);
        assert!(*up_counter.borrow() == 1);
        assert!(keyboard.events.is_empty());

        keyboard.add_keypress(KeyCode::A, 20);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 2);
        assert!(*up_counter.borrow() == 1);

        keyboard.add_keyrelease(KeyCode::A, 20);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 2);
        assert!(*up_counter.borrow() == 2);

        //third release - no change
        keyboard.add_keyrelease(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 2);
        assert!(*up_counter.borrow() == 2);
        assert!(keyboard.events.is_empty());

        //fourth press - sets
        keyboard.add_keypress(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 3);
        assert!(*up_counter.borrow() == 2);
        assert!(keyboard.events.is_empty());
    }

    #[test]
    fn test_press_release() {
        let down_counter = RefCell::new(0);
        let up_counter = RefCell::new(0);
        let t = PressReleaseMacro::new(
            0xF0000u32,
            Option::Some(|output: &mut KeyOutCatcher| {
                //todo: why do we need to define the type here?
                output.send_keys(&[KeyCode::H]);
                let mut dc = down_counter.borrow_mut();
                *dc += 1;
            }),
            Option::Some(|output: &mut KeyOutCatcher| {
                let mut dc = up_counter.borrow_mut();
                *dc += 1;
                output.send_keys(&[KeyCode::I]);
            }),
        );
        let h = vec![
            Box::new(t) as Box<dyn ProcessKeys<KeyOutCatcher>>,
            Box::new(USBKeyboard::new()),
        ];

        let mut keyboard = Keyboard::new(h, KeyOutCatcher::new());
        //first press - sets
        keyboard.add_keypress(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 0);
        assert!(keyboard.events.is_empty());
        check_output(&keyboard, &[&[KeyCode::H], &[]]);
        keyboard.output.clear();

        //first release - no change
        keyboard.add_keyrelease(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 1);
        check_output(&keyboard, &[&[KeyCode::I], &[]]);
        keyboard.output.clear();
    }

    #[test]
    fn test_layer_rewrite() {
        let l = Layer::new(vec![(
            KeyCode::A,
            LayerAction::RewriteTo(KeyCode::X.into()),
        )]);
        let h = vec![
            Box::new(l) as Box<dyn ProcessKeys<KeyOutCatcher>>,
            Box::new(USBKeyboard::new()),
        ];

        let mut keyboard = Keyboard::new(h, KeyOutCatcher::new());
        keyboard.add_keypress(KeyCode::B, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::B, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();

        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();

        check_output(
            &keyboard,
            &[&[KeyCode::B], &[], &[KeyCode::X], &[], &[KeyCode::X], &[]],
        );

        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keypress(KeyCode::B, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::B, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(
            &keyboard,
            &[&[KeyCode::X], &[KeyCode::X, KeyCode::B], &[KeyCode::X], &[]],
        );

        keyboard.output.clear();
        keyboard.handlers[0].disable();
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::A], &[]]);

        keyboard.output.clear();
        keyboard.handlers[0].enable();
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::X], &[]]);

        //TODO: what happens when you disable the layer in the middle?
        // I suspect that we will keep repeating one of the keycodes.
        // what would be the sensible thing to happen? How can we achive this?
        // possibly by clearing the keyboard events whenever a layer toggle happens?
    }

    #[test]
    fn test_tapdance() {
        let l = TapDance::new(
            KeyCode::X,
            |tap_count, output: &mut KeyOutCatcher| match tap_count {
                1 => output.send_keys(&[KeyCode::A]),
                2 => output.send_keys(&[KeyCode::B]),
                3 => output.send_keys(&[KeyCode::C]),
                _ => output.send_keys(&[KeyCode::D]),
            },
        );
        let timeout = l.timeout_ms;
        let h = vec![
            Box::new(l) as Box<dyn ProcessKeys<KeyOutCatcher>>,
            Box::new(USBKeyboard::new()),
        ];

        let mut keyboard = Keyboard::new(h, KeyOutCatcher::new());

        //simplest case, one press/release then another key
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 10);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::Z, 20);
        keyboard.handle_keys().unwrap();
        //       keyboard.add_keyrelease(KeyCode::Z, 30);
        check_output(&keyboard, &[&[KeyCode::A], &[KeyCode::Z]]);
        keyboard.add_keyrelease(KeyCode::Z, 20);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.events.is_empty());

        //two taps, then another key
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 10);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::X, 20);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 30);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::Z, 40);
        keyboard.handle_keys().unwrap();
        //        keyboard.add_keyrelease(KeyCode::Z, 50);
        check_output(&keyboard, &[&[KeyCode::B], &[KeyCode::Z]]);
        keyboard.add_keyrelease(KeyCode::Z, 20);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.events.is_empty());

        //three taps, then a time out
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 10);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::X, 20);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 30);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::X, 20);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 30);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_timeout(timeout - 1);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_timeout(timeout + 1);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::C], &[]]);
    }

    #[test]
    fn test_sticky_macro() {
        let l = StickyMacro::new(
            KeyCode::X,
            |output: &mut KeyOutCatcher| output.send_keys(&[KeyCode::A]),
            |output: &mut KeyOutCatcher| output.send_keys(&[KeyCode::B]),
        );
        let h = vec![
            Box::new(l) as Box<dyn ProcessKeys<KeyOutCatcher>>,
            Box::new(USBKeyboard::new()),
        ];

        let mut keyboard = Keyboard::new(h, KeyOutCatcher::new());

        //activate
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::A], &[]]);
        keyboard.output.clear();

        //ignore
        keyboard.add_keyrelease(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        //ignore
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        //deactivate
        keyboard.add_keyrelease(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::B], &[]]);
    }

    #[test]
    fn test_leader() {
        use crate::key_codes::KeyCode::*;
        use core::convert::TryInto;

        let mut l = Leader::new(
            KeyCode::X,
            vec![
                (vec![A, B, C], "A"),
                (vec![A, B, D], "B"),
                //Todo: check that none is a prefix of another!
                //(vec![A], "C"),
            ],
            "E",
        );
        assert!(l.match_prefix() == MatchResult::NeedsMoreInput);
        l.prefix.push(A.into());
        assert!(l.match_prefix() == MatchResult::NeedsMoreInput);
        l.prefix.push(B.into());
        assert!(l.match_prefix() == MatchResult::NeedsMoreInput);
        l.prefix.push(C.into());
        assert!(match l.match_prefix() {
            MatchResult::Match(m) => {
                assert!(m == "A");
                true
            }
            _ => false,
        });
        l.prefix.clear();
        assert!(l.match_prefix() == MatchResult::NeedsMoreInput);
        l.prefix.push(C.into());
        assert!(l.match_prefix() == MatchResult::WontMatch);
        l.prefix.clear();

        let keyb = USBKeyboard::new();
        let h = vec![
            Box::new(l) as Box<dyn ProcessKeys<KeyOutCatcher>>,
            Box::new(keyb),
        ];

        let mut keyboard = Keyboard::new(h, KeyOutCatcher::new());
        keyboard.output.set_unicode_mode(UnicodeSendMode::Debug);

        //activate
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::B, 0);
        keyboard.add_keyrelease(KeyCode::B, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::C, 0);
        keyboard.add_keyrelease(KeyCode::C, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[65u8.try_into().unwrap()], &[]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::F, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::F, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[F], &[]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        //test error case
        keyboard.add_keypress(KeyCode::C, 0);
        keyboard.add_keyrelease(KeyCode::C, 0);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[69u8.try_into().unwrap()], &[]]);
    }

    #[test]
    fn test_spacecadet() {
        let down_counter = RefCell::new(0);
        let up_counter = RefCell::new(0);
        let l = SpaceCadet::new(
            KeyCode::X,
            |output: &mut KeyOutCatcher| {
                println!("activate");
                let mut c = down_counter.borrow_mut();
                *c += 1;
                output.send_keys(&[KeyCode::A])
            },
            |output: &mut KeyOutCatcher| {
                println!("deactivate");
                let mut c = up_counter.borrow_mut();
                *c += 1;
                output.send_keys(&[KeyCode::B])
            },
        );
        let h = vec![
            Box::new(l) as Box<dyn ProcessKeys<KeyOutCatcher>>,
            Box::new(USBKeyboard::new()),
        ];

        let mut keyboard = Keyboard::new(h, KeyOutCatcher::new());

        //the tap...
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 10);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::X]]);
        keyboard.output.clear();

        assert!(keyboard.events.is_empty());

        //the modifier
        println!("as mod");
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::Z, 0);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[KeyCode::A], &[KeyCode::Z]]);
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 0);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::Z, 0);
        keyboard.handle_keys().unwrap();
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 10);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::B], &[]]);
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 1);
        keyboard.output.clear();
    }

    #[test]
    fn test_autoshift() {
        let threshold = 200;
        let l = AutoShift::new(threshold);
        let h = vec![
            Box::new(l) as Box<dyn ProcessKeys<KeyOutCatcher>>,
            Box::new(USBKeyboard::new()),
        ];

        let mut keyboard = Keyboard::new(h, KeyOutCatcher::new());

        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, threshold-1);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::X], &[]]);
        keyboard.output.clear();
        dbg!(&keyboard.events);
        assert!(keyboard.events.is_empty());

        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, threshold+1);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[KeyCode::X, KeyCode::LShift], &[]]);
        keyboard.output.clear();

    }
    #[test]
    fn test_autoshift_no_letters() {
        let threshold = 200;
        let mut l = AutoShift::new(threshold);
        l.shift_letters=false;
        let h = vec![
            Box::new(l) as Box<dyn ProcessKeys<KeyOutCatcher>>,
            Box::new(USBKeyboard::new()),
        ];

        let mut keyboard = Keyboard::new(h, KeyOutCatcher::new());

        keyboard.add_keypress(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::Kb1, threshold-1);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::Kb1], &[]]);
        keyboard.output.clear();
        dbg!(&keyboard.events);
        assert!(keyboard.events.is_empty());

        keyboard.add_keypress(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::Kb1, threshold+1);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[KeyCode::Kb1, KeyCode::LShift], &[]]);
        keyboard.output.clear();

        //this now get's handled by the usb keyboard.
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::X]]);
        keyboard.output.clear()
    }

    #[test]
    fn test_modifiers() {
        let h = vec![
            Box::new(USBKeyboard::new()) as Box<dyn ProcessKeys<KeyOutCatcher>>
        ];

        let mut keyboard = Keyboard::new(h, KeyOutCatcher::new());

        keyboard.add_keypress(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::Kb1]]);

        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.state.shift = true;

        keyboard.add_keypress(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::Kb1, KeyCode::LShift]]);

        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift]]);
        keyboard.output.clear();

        keyboard.state.shift = false;
        keyboard.state.ctrl = true;

        keyboard.add_keypress(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::Kb1, KeyCode::LCtrl]]);

        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LCtrl]]);
        keyboard.output.clear();

        keyboard.state.ctrl = false;
        keyboard.state.alt = true;

        keyboard.add_keypress(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::Kb1, KeyCode::LAlt]]);

        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LAlt]]);
        keyboard.output.clear();

        keyboard.state.alt = false;
        keyboard.state.meta = true;

        keyboard.add_keypress(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::Kb1, KeyCode::LGui]]);

        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LGui]]);


    }
 
}
