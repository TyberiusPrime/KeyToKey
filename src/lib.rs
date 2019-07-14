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

#[derive(PartialEq, Debug)]
struct TimeOut {
    ms_since_last: u16,
}

pub const UNICODE_BELOW_256: u32 = 0x100000;

#[derive(PartialEq, Debug)]
struct KeyPress {
    keycode: u32,
    ms_since_last: u16,
    running_number: u8,
}

impl KeyPress {
    fn new(keycode: u32) -> KeyPress {
        KeyPress {
            keycode,
            ms_since_last: 0,
            running_number: 0,
        }
    }
}

#[derive(PartialEq, Debug)]
struct KeyRelease {
    keycode: u32,
    ms_since_last: u16,
    running_number: u8,
} //Timeout

impl KeyRelease {
    fn new(keycode: u32) -> KeyRelease {
        KeyRelease {
            keycode,
            ms_since_last: 0,
            running_number: 0,
        }
    }
}

#[derive(PartialEq, Debug)]
enum Event {
    KeyPress(KeyPress),
    KeyRelease(KeyRelease),
    TimeOut(TimeOut),
}

#[derive(PartialEq, Debug)]
enum EventStatus {
    Unhandled,
    Handled,
    Ignored,
}

impl Event {
    fn is_key_press(&self, keycode: u32) -> bool {
        match self {
            Event::KeyPress(kc) => return kc.keycode == keycode,
            _ => false,
        }
    }
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

struct Input<'a, T: USBKeyOut> {
    events: Vec<(Event, EventStatus)>,
    running_number: u8,
    handlers: Vec<Box<dyn ProcessKeys<T> + 'a>>,
    output: T,
}

impl<'a, T: USBKeyOut> Input<'_, T> {
    fn new(handlers: Vec<Box<dyn ProcessKeys<T> + 'a>>, output: T) -> Input<T> {
        Input {
            events: Vec::new(),
            running_number: 0,
            handlers,
            output,
        }
    }

    fn handle_keys(&mut self) -> Result<(), String> {
        for (_e, status) in self.events.iter_mut() {
            *status = EventStatus::Unhandled;
        }
        for h in self.handlers.iter_mut() {
            h.process_keys(&mut self.events, &mut self.output);
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
        let e = KeyPress {
            keycode: keycode.to_u32(),
            ms_since_last,
            running_number: self.running_number,
        };
        self.running_number += 1;
        self.events
            .push((Event::KeyPress(e), EventStatus::Unhandled));
    }
    fn add_keyrelease<X: AcceptsKeycode>(&mut self, keycode: X, ms_since_last: u16) {
        let e = KeyRelease {
            keycode: keycode.to_u32(),
            ms_since_last,
            running_number: self.running_number,
        };
        self.running_number += 1;
        self.events
            .push((Event::KeyRelease(e), EventStatus::Unhandled));
    }

    fn add_timeout(&mut self, ms_since_last: u16) {
        let e = TimeOut { ms_since_last };
        if let Some((event, _status)) = self.events.iter().last() {
            if let Event::TimeOut(_) = event {
                self.events.pop();
            }
        }
        self.events
            .push((Event::TimeOut(e), EventStatus::Unhandled));
    }
}

#[derive(Clone, Copy)]
enum UnicodeSendMode {
    Linux = 1,
    WinCompose,
}

trait ProcessKeys<T: USBKeyOut> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> ();
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
    shift: bool,
    ctrl: bool,
    alt: bool,
    meta: bool,
    unicode_sendmode: UnicodeSendMode,
}

impl USBKeyboard {
    fn new() -> USBKeyboard {
        USBKeyboard {
            shift: false,
            ctrl: false,
            alt: false,
            meta: false,
            unicode_sendmode: UnicodeSendMode::Linux,
        }
    }
}
impl<T: USBKeyOut> ProcessKeys<T> for USBKeyboard {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
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
                    if codes_to_delete.contains(&kc.keycode) {
                        *status = EventStatus::Handled;
                    } else {
                        if kc.keycode < 256 {
                            let oc: Result<KeyCode, String> = (kc.keycode as u8).try_into();
                            match oc {
                                Ok(x) => {
                                    output.register_key(x);
                                    *status = EventStatus::Ignored; //so we may resend it...
                                }
                                Err(_) => *status = EventStatus::Handled, //throw it away, will ya?
                            };
                        }
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
        //dbg!(&result);
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
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
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

/*struct LeaderKeyboard <'a>{
    leader: u32,
    mappings: &'a[(&'a[u32], String)],
    on_failure: fn () //todo
}

impl<T: USBKeyOut> ProcessKeys<T> for LeaderKeyboard {
    fn process_keys(&mut self, input: &mut Vec<Event>, output: &mut T) -> ProcessingResult {
    let mut result = ProcessingResult::NotMine;
    let mut inside_leader_sequence = false;
    let mut Vec<Event> sequence = Vec::new();
    for k in input.iter_mut(){
        if !inside_leader_sequence{
        match k {
           Event::KeyRelease(kc) => if kc == self.leader {
               result = ProcessingResult::NeedMoreInput;
           }
           Event::KeyPress(kc) => if kc == self.leader {
               *k = Event::Deleted;
           }
           _ => {}
        }
        } else
        {
        match k {
           Event::KeyRelease(kc) => if kc == self.leader {
               result = ProcessingResult::NeedMoreInput;
               sequence.push(kc);
               let hit = self.match_sequence(sequence)
               match hit {
                   LeaderSequence::Hit(s) {output.send_string(s); return ProcessingResult.Processed},
                   LeaderSequence::Miss() => {self.on_failure(); input.clear(); return ProcessingResult.Processed;}
                   LeaderSequence::Wait() => {return ProcessingResult.NeedMoreInput}
               }
           }
           Event::KeyPress(kc) => if kc == self.leader {
               *k = Event::Deleted;
           }
           _ => {}

        }
    }
    return result;
}
}
*/

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
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
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
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
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
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
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
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
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
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
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
                Event::TimeOut(t) => {
                    if self.tap_count > 0 && t.ms_since_last > self.timeout_ms {
                        (self.on_tap_complete)(self.tap_count, output);
                        self.tap_count = 0;
                    }
                }
            }
        }
    }
}

//todo:
// one shot: engage on first keypress.
//   disengage after any subsequent keyrelease.
//   except if they key is still being pressed
//   if that happend, disengage once the
//   trigger get's released

// space cadet keys: one thing on tap, modifier/macro on press
// finish leader key

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
        Event, Input, Layer, LayerAction, OneShot, PressReleaseMacro, ProcessKeys, StickyMacro,
        TapDance, USBKeyOut, USBKeyboard, UnicodeKeyboard, UnicodeSendMode, UNICODE_BELOW_256,
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
    fn check_output(input: &Input<KeyOutCatcher>, should: &[&[KeyCode]]) {
        assert!(should.len() == input.output.reports.len());
        for (ii, report) in should.iter().enumerate() {
            assert!(input.output.reports[ii].len() == report.len());
            for k in report.iter() {
                let kcu: u8 = (*k).into();
                assert!(input.output.reports[ii].contains(&kcu));
            }
        }
    }
    #[test]
    fn test_usbkeyboard_single_key() {
        let h = vec![Box::new(USBKeyboard::new()) as Box<dyn ProcessKeys<KeyOutCatcher>>];
        let mut input = Input::new(h, KeyOutCatcher::new());
        input.add_keypress(KeyCode::A, 0);
        input.handle_keys().unwrap();
        check_output(&input, &[&[KeyCode::A]]);
        assert!(!input.events.is_empty());
        input.add_keyrelease(KeyCode::A, 20);
        assert!(input.events.len() == 2);
        input.output.clear();
        input.handle_keys().unwrap();
        check_output(&input, &[&[]]);
        assert!(input.events.is_empty());
    }
    #[test]
    fn test_usbkeyboard_multiple_key() {
        use KeyCode::*;
        let h = vec![Box::new(USBKeyboard::new()) as Box<dyn ProcessKeys<KeyOutCatcher>>];
        let mut input = Input::new(h, KeyOutCatcher::new());
        input.add_keypress(A, 0);
        input.handle_keys().unwrap();
        check_output(&input, &[&[A]]);
        assert!(!input.events.is_empty());

        input.output.clear();
        input.add_keypress(KeyCode::X, 0);
        input.handle_keys().unwrap();
        check_output(&input, &[&[A, X]]);
        assert!(!input.events.is_empty());

        input.output.clear();
        input.add_keyrelease(KeyCode::A, 20);
        assert!(input.events.len() == 3);
        input.handle_keys().unwrap();
        check_output(&input, &[&[X]]);
        assert!(!input.events.is_empty());

        input.output.clear();
        input.add_keyrelease(KeyCode::X, 20);
        assert!(input.events.len() == 2);
        input.handle_keys().unwrap();
        check_output(&input, &[&[]]);
        assert!(input.events.is_empty());
    }

    #[test]
    fn test_unicode_keyboard_linux() {
        use KeyCode::*;
        let ub = UnicodeKeyboard {};
        let h = vec![Box::new(ub) as Box<dyn ProcessKeys<KeyOutCatcher>>];
        let mut input = Input::new(h, KeyOutCatcher::new());
        input.output.set_unicode_mode(UnicodeSendMode::Linux);
        //no output on press
        input.add_keypress(0x00E4u32 + UNICODE_BELOW_256, 0);
        input.handle_keys().unwrap();
        assert!(input.output.reports.len() == 0);
        assert!(input.events.is_empty()); // we eat the keypress though

        input.add_keyrelease(0x00E4 + UNICODE_BELOW_256, 0);
        input.handle_keys().unwrap();
        check_output(
            &input,
            &[
                &[U, LShift, LCtrl],
                &[E, LShift, LCtrl],
                &[Kb4, LShift, LCtrl],
                &[],
            ],
        );

        assert!(input.events.is_empty()); // we eat the keypress though
    }

    #[test]
    fn test_unicode_keyboard_wincompose() {
        use KeyCode::*;
        let ub = UnicodeKeyboard {};
        let h = vec![Box::new(ub) as Box<dyn ProcessKeys<KeyOutCatcher>>];
        let mut input = Input::new(h, KeyOutCatcher::new());
        input.output.set_unicode_mode(UnicodeSendMode::WinCompose);
        //no output on press
        input.add_keypress(0x03B4u32, 0);
        input.handle_keys().unwrap();
        assert!(input.output.reports.len() == 0);
        assert!(input.events.is_empty()); // we eat the keypress though

        input.add_keyrelease(0x03B4, 0);
        input.handle_keys().unwrap();
        check_output(&input, &[&[RAlt], &[U], &[Kb3], &[B], &[Kb4], &[]]);
        assert!(input.events.is_empty()); // we eat the keypress though
    }

    #[test]
    fn test_unicode_while_depressed() {
        use KeyCode::*;
        let h = vec![
            Box::new(UnicodeKeyboard {}) as Box<dyn ProcessKeys<KeyOutCatcher>>,
            Box::new(USBKeyboard::new()),
        ];
        let mut input = Input::new(h, KeyOutCatcher::new());
        input.output.set_unicode_mode(UnicodeSendMode::WinCompose);
        input.add_keypress(A, 0);
        input.handle_keys().unwrap();
        check_output(&input, &[&[A]]);
        input.output.clear();
        input.add_keypress(0x3B4u32, 0);
        input.handle_keys().unwrap();
        check_output(&input, &[&[A]]);
        input.add_keyrelease(0x3B4, 0);
        input.output.clear();
        input.handle_keys().unwrap();
        check_output(&input, &[&[RAlt], &[U], &[Kb3], &[B], &[Kb4], &[], &[A]]);
        input.add_keyrelease(A, 0);
        input.output.clear();
        input.handle_keys().unwrap();
        check_output(&input, &[&[]]);
        assert!(input.events.is_empty());
    }

    #[test]
    fn test_panic_on_unhandled() {
        let h = vec![Box::new(USBKeyboard::new()) as Box<dyn ProcessKeys<KeyOutCatcher>>];
        let mut input = Input::new(h, KeyOutCatcher::new());
        input.add_keypress(0xF0000u32, 0);
        assert!(input.handle_keys().is_err());
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

        let mut input = Input::new(h, KeyOutCatcher::new());
        //first press - sets
        input.add_keypress(0xF0000u32, 0);
        input.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 0);
        assert!(input.events.is_empty());
        check_output(&input, &[&[KeyCode::H], &[]]);
        input.output.clear();

        //first release - no change
        input.add_keyrelease(0xF0000u32, 0);
        input.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 0);
        assert!(input.events.is_empty());

        //second press - unsets
        input.add_keypress(0xF0000u32, 0);
        input.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 1);
        assert!(input.events.is_empty());

        //second release - no change
        input.add_keyrelease(0xF0000u32, 0);
        input.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 1);
        assert!(input.events.is_empty());

        //third press - sets
        input.add_keypress(0xF0000u32, 0);
        input.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 2);
        assert!(*up_counter.borrow() == 1);
        assert!(input.events.is_empty());

        input.add_keypress(KeyCode::A, 20);
        input.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 2);
        assert!(*up_counter.borrow() == 1);

        input.add_keyrelease(KeyCode::A, 20);
        input.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 2);
        assert!(*up_counter.borrow() == 2);

        //third release - no change
        input.add_keyrelease(0xF0000u32, 0);
        input.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 2);
        assert!(*up_counter.borrow() == 2);
        assert!(input.events.is_empty());

        //fourth press - sets
        input.add_keypress(0xF0000u32, 0);
        input.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 3);
        assert!(*up_counter.borrow() == 2);
        assert!(input.events.is_empty());
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

        let mut input = Input::new(h, KeyOutCatcher::new());
        //first press - sets
        input.add_keypress(0xF0000u32, 0);
        input.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 0);
        assert!(input.events.is_empty());
        check_output(&input, &[&[KeyCode::H], &[]]);
        input.output.clear();

        //first release - no change
        input.add_keyrelease(0xF0000u32, 0);
        input.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 1);
        check_output(&input, &[&[KeyCode::I], &[]]);
        input.output.clear();
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

        let mut input = Input::new(h, KeyOutCatcher::new());
        input.add_keypress(KeyCode::B, 0);
        input.handle_keys().unwrap();
        input.add_keyrelease(KeyCode::B, 0);
        input.handle_keys().unwrap();
        input.add_keypress(KeyCode::A, 0);
        input.handle_keys().unwrap();
        input.add_keyrelease(KeyCode::A, 0);
        input.handle_keys().unwrap();

        input.add_keypress(KeyCode::X, 0);
        input.handle_keys().unwrap();
        input.add_keyrelease(KeyCode::X, 0);
        input.handle_keys().unwrap();

        check_output(
            &input,
            &[&[KeyCode::B], &[], &[KeyCode::X], &[], &[KeyCode::X], &[]],
        );

        input.output.clear();
        input.add_keypress(KeyCode::A, 0);
        input.handle_keys().unwrap();
        input.add_keypress(KeyCode::B, 0);
        input.handle_keys().unwrap();
        input.add_keyrelease(KeyCode::B, 0);
        input.handle_keys().unwrap();
        input.add_keyrelease(KeyCode::A, 0);
        input.handle_keys().unwrap();
        check_output(
            &input,
            &[&[KeyCode::X], &[KeyCode::X, KeyCode::B], &[KeyCode::X], &[]],
        );

        input.output.clear();
        input.handlers[0].disable();
        input.add_keypress(KeyCode::A, 0);
        input.handle_keys().unwrap();
        input.add_keyrelease(KeyCode::A, 0);
        input.handle_keys().unwrap();
        check_output(&input, &[&[KeyCode::A], &[]]);

        input.output.clear();
        input.handlers[0].enable();
        input.add_keypress(KeyCode::A, 0);
        input.handle_keys().unwrap();
        input.add_keyrelease(KeyCode::A, 0);
        input.handle_keys().unwrap();
        check_output(&input, &[&[KeyCode::X], &[]]);

        //TODO: what happens when you disable the layer in the middle?
        // I suspect that we will keep repeating one of the keycodes.
        // what would be the sensible thing to happen? How can we achive this?
        // possibly by clearing the input events whenever a layer toggle happens?
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

        let mut input = Input::new(h, KeyOutCatcher::new());

        //simplest case, one press/release then another key
        input.add_keypress(KeyCode::X, 0);
        input.handle_keys().unwrap();
        check_output(&input, &[&[]]);
        input.output.clear();

        input.add_keyrelease(KeyCode::X, 10);
        input.handle_keys().unwrap();
        check_output(&input, &[&[]]);
        input.output.clear();

        input.add_keypress(KeyCode::Z, 20);
        input.handle_keys().unwrap();
        //       input.add_keyrelease(KeyCode::Z, 30);
        check_output(&input, &[&[KeyCode::A], &[KeyCode::Z]]);
        input.add_keyrelease(KeyCode::Z, 20);
        input.handle_keys().unwrap();
        assert!(input.events.is_empty());

        //two taps, then another key
        input.output.clear();
        input.add_keypress(KeyCode::X, 0);
        input.handle_keys().unwrap();
        check_output(&input, &[&[]]);
        input.output.clear();

        input.add_keyrelease(KeyCode::X, 10);
        input.handle_keys().unwrap();
        check_output(&input, &[&[]]);
        input.output.clear();

        input.add_keypress(KeyCode::X, 20);
        input.handle_keys().unwrap();
        check_output(&input, &[&[]]);
        input.output.clear();

        input.add_keyrelease(KeyCode::X, 30);
        input.handle_keys().unwrap();
        check_output(&input, &[&[]]);
        input.output.clear();

        input.add_keypress(KeyCode::Z, 40);
        input.handle_keys().unwrap();
        //        input.add_keyrelease(KeyCode::Z, 50);
        check_output(&input, &[&[KeyCode::B], &[KeyCode::Z]]);
        input.add_keyrelease(KeyCode::Z, 20);
        input.handle_keys().unwrap();
        assert!(input.events.is_empty());

        //three taps, then a time out
        input.output.clear();
        input.add_keypress(KeyCode::X, 0);
        input.handle_keys().unwrap();
        check_output(&input, &[&[]]);
        input.output.clear();

        input.add_keyrelease(KeyCode::X, 10);
        input.handle_keys().unwrap();
        check_output(&input, &[&[]]);
        input.output.clear();

        input.add_keypress(KeyCode::X, 20);
        input.handle_keys().unwrap();
        check_output(&input, &[&[]]);
        input.output.clear();

        input.add_keyrelease(KeyCode::X, 30);
        input.handle_keys().unwrap();
        check_output(&input, &[&[]]);
        input.output.clear();

        input.add_keypress(KeyCode::X, 20);
        input.handle_keys().unwrap();
        check_output(&input, &[&[]]);
        input.output.clear();

        input.add_keyrelease(KeyCode::X, 30);
        input.handle_keys().unwrap();
        check_output(&input, &[&[]]);
        input.output.clear();

        input.add_timeout(timeout - 1);
        input.handle_keys().unwrap();
        check_output(&input, &[&[]]);
        input.output.clear();

        input.add_timeout(timeout + 1);
        input.handle_keys().unwrap();
        check_output(&input, &[&[KeyCode::C], &[]]);
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

        let mut input = Input::new(h, KeyOutCatcher::new());

        //activate
        input.add_keypress(KeyCode::X, 0);
        input.handle_keys().unwrap();
        check_output(&input, &[&[KeyCode::A], &[]]);
        input.output.clear();

        //ignore
        input.add_keyrelease(KeyCode::X, 0);
        input.handle_keys().unwrap();
        check_output(&input, &[&[]]);
        input.output.clear();

        //ignore
        input.add_keypress(KeyCode::X, 0);
        input.handle_keys().unwrap();
        check_output(&input, &[&[]]);
        input.output.clear();

        //deactivate
        input.add_keyrelease(KeyCode::X, 0);
        input.handle_keys().unwrap();
        check_output(&input, &[&[KeyCode::B], &[]]);
    }

}
