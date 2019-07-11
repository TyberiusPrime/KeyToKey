#![allow(dead_code)]
#![feature(drain_filter)]
#![no_std]

mod key_codes;

extern crate alloc;
extern crate no_std_compat;

use crate::key_codes::KeyCode;
use core::convert::TryInto;
use no_std_compat::prelude::v1::*;

/*struct TimeOut {
    ms_since_last: u16,
}

*/
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
}

#[derive(PartialEq, Debug)]
enum EventStatus{
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


fn iter_unhandled_mut(events: &mut Vec<(Event, EventStatus)>) -> impl DoubleEndedIterator<Item=&mut (Event, EventStatus)>
{
    events.iter_mut().filter(|(_e, status)| EventStatus::Unhandled == *status)
}


struct  Input <'a, T: USBKeyOut> {
    events: Vec<(Event, EventStatus)>,
    running_number: u8,
    handlers: &'a mut [Box<dyn ProcessKeys<T>>],
    output: T,
}

impl <T: USBKeyOut> Input<'_, T>{
    fn new(handlers: &mut [Box<dyn ProcessKeys<T>>], output: T) -> Input<T> {
        Input {
            events: Vec::new(),
            running_number: 0,
            handlers,
            output
        }
    }

    fn handle_keys(
        &mut self,
    ) {
        for (_e, status) in self.events.iter_mut() {
            *status = EventStatus::Unhandled;
        }
        for h in self.handlers.iter_mut() {
            h.process_keys(&mut self.events, &mut self.output);
                    }
        self.events.drain_filter(|(_e, status)| EventStatus::Handled == *status);
        if self.events.iter().any(|(_e, status)| EventStatus::Unhandled == *status){
            panic!("Unhandled input! {:?}", self.events);
        }
    }



    fn add_keypress(&mut self, keycode: u32, ms_since_last: u16) {
        let e = KeyPress {
            keycode,
            ms_since_last,
            running_number: self.running_number,
        };
        self.running_number += 1;
        self.events.push((Event::KeyPress(e), EventStatus::Unhandled));
    }
    fn add_keyrelease(&mut self, keycode: u32, ms_since_last: u16) {
        let e = KeyRelease {
            keycode,
            ms_since_last,
            running_number: self.running_number,
        };
        self.running_number += 1;
        self.events.push((Event::KeyRelease(e), EventStatus::Unhandled));
    }
}

#[derive(Clone, Copy)]
enum UnicodeSendMode {
    Linux = 1,
    WinCompose,
}

trait ProcessKeys<T: USBKeyOut> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>,  output: &mut T) -> ();
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
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>,  output: &mut T) -> () {
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
                                },
                                Err(_) => *status = EventStatus::Handled, //throw it away, will ya?
                            };
                        }
                    }
                }
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
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>,  output: &mut T) -> (){
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
/*
enum LayerAction<'a, T: USBKeyOut> {
    RewriteTo(u32),
    SendString(&'a str),
    Callback(fn(&mut T) -> (), fn(&mut T) -> ()),
}
struct Layer<'a, T: USBKeyOut> {
    rewrites: &'a [(u32, LayerAction<'a, T>)],
}
impl<T: USBKeyOut> ProcessKeys<T> for Layer<'_, T> {
    fn process_keys(&mut self, input: &mut Input, output: &mut T) -> ProcessingResult {
        for k in input.events.iter_mut() {
            match k {
                Event::KeyRelease(kc) => {
                    for (from, to) in self.rewrites.iter() {
                        if *from == kc.keycode {
                            match to {
                                LayerAction::RewriteTo(to_keycode) => {
                                    kc.keycode = *to_keycode;
                                    return ProcessingResult::NotMine;
                                }
                                LayerAction::SendString(s) => {
                                    output.send_string(s);
                                    *k = Event::Deleted;
                                    return ProcessingResult::Processed;
                                }
                                LayerAction::Callback(_, on_release) => {
                                    on_release(output);
                                    *k = Event::Deleted;
                                    return ProcessingResult::Processed;
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
                                    return ProcessingResult::NotMine;
                                }
                                LayerAction::Callback(on_press, _) => {
                                    on_press(output);
                                    *k = Event::Deleted;
                                    return ProcessingResult::Processed;
                                }

                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        ProcessingResult::NotMine
    }
}

struct PressRelaseMacro<T: USBKeyOut> {
    keycode: u32,
    pub on_press: fn(&mut T) -> (),
    pub on_release: fn(&mut T) -> (),
}

impl<T: USBKeyOut> ProcessKeys<T> for PressRelaseMacro<T> {
    fn process_keys(&mut self, input: &mut Input, output: &mut T) -> ProcessingResult {
        for k in input.events.iter_mut() {
            match k {
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.keycode {
                        (self.on_press)(output);
                        *k = Event::Deleted;
                        return ProcessingResult::Processed;
                    }
                }
                Event::KeyPress(kc) => {
                    if kc.keycode == self.keycode {
                        (self.on_release)(output);
                        *k = Event::Deleted;
                        return ProcessingResult::Processed;
                    }
                }
                _ => {}
            }
        }
        ProcessingResult::NotMine
    }
}

/// a macro that is called 'on' on the the first keypress
/// and off on the second keyrelease.
/// Using this you can implement e.g. sticky modifiers
struct ToggleMacro<T: USBKeyOut> {
    keycode: u32,
    on_toggle_on: fn(&mut T) -> (),
    on_toggle_off: fn(&mut T) -> (),
    state: bool,
}
impl<T: USBKeyOut> ToggleMacro<T> {
    fn new(
        trigger: u32,
        on_toggle_on: fn(&mut T) -> (),
        on_toggle_off: fn(&mut T) -> (),
    ) -> ToggleMacro<T> {
        ToggleMacro {
            keycode: trigger,
            on_toggle_on,
            on_toggle_off,
            state: false,
        }
    }
}
impl<T: USBKeyOut> ProcessKeys<T> for ToggleMacro<T> {
    fn process_keys(&mut self, input: &mut Input, output: &mut T) -> ProcessingResult {
        for k in input.events.iter_mut() {
            match k {
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.keycode {
                        *k = Event::Deleted;
                        if !self.state {
                            (self.on_toggle_on)(output);
                            self.state = !self.state;
                        }
                        return ProcessingResult::Processed;
                    }
                }
                Event::KeyPress(kc) => {
                    if kc.keycode == self.keycode {
                        *k = Event::Deleted;
                        if self.state {
                            (self.on_toggle_off)(output);
                            self.state = !self.state;
                        }
                        return ProcessingResult::Processed;
                    }
                }
                _ => {}
            }
        }
        ProcessingResult::NotMine
    }
}
*/
//todo:
// one shot: engage on first keypress.
//   disengage after any subsequent keyrelease.
//   except if they key is still being pressed
//   if that happend, disengage once the
//   trigger get's released

// space cadet keys: one thing on tap, modifier/macro on press
// finish leader key
// tap dance

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
        Event, Input, ProcessKeys, USBKeyOut, USBKeyboard, UnicodeKeyboard,
        UnicodeSendMode, UNICODE_BELOW_256,
    };
    use no_std_compat::prelude::v1::*;
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
        let mut h = [Box::new(USBKeyboard::new()) as Box<dyn ProcessKeys<KeyOutCatcher>>];
        let mut input = Input::new(&mut h, KeyOutCatcher::new());
        input.add_keypress(KeyCode::A.into(), 0);
        input.handle_keys();
        check_output(&input, &[&[KeyCode::A]]);
        assert!(!input.events.is_empty());
        input.add_keyrelease(KeyCode::A.into(), 20);
        assert!(input.events.len() == 2);
        input.output.clear();
        input.handle_keys();
        check_output(&input, &[&[]]);
        dbg!(&input.events);
        assert!(input.events.is_empty());
    }
    #[test]
    fn test_usbkeyboard_multiple_key() {
        use KeyCode::*;
        let mut h = [Box::new(USBKeyboard::new()) as Box<dyn ProcessKeys<KeyOutCatcher>>];
        let mut input = Input::new(&mut h, KeyOutCatcher::new());
        input.add_keypress(A.into(), 0);
        input.handle_keys();
        dbg!(&input.output.reports);
        check_output(&input, &[&[A]]);
        assert!(!input.events.is_empty());

        input.output.clear();
        input.add_keypress(KeyCode::X.into(), 0);
        input.handle_keys();
        check_output(&input, &[&[A, X]]);
        assert!(!input.events.is_empty());

        input.output.clear();
        input.add_keyrelease(KeyCode::A.into(), 20);
        assert!(input.events.len() == 3);
        input.handle_keys();
        check_output(&input, &[&[X]]);
        assert!(!input.events.is_empty());

        input.output.clear();
        input.add_keyrelease(KeyCode::X.into(), 20);
        assert!(input.events.len() == 2);
        input.handle_keys();
        check_output(&input, &[&[]]);
        assert!(input.events.is_empty());
    }

    

    #[test]
    fn test_unicode_keyboard_linux() {
        use KeyCode::*;
        let ub = UnicodeKeyboard {};
        let mut h = [Box::new(ub) as Box<dyn ProcessKeys<KeyOutCatcher>>];
        let mut input = Input::new(&mut h, KeyOutCatcher::new());
        input.output.set_unicode_mode(UnicodeSendMode::Linux);
        //no output on press
        input.add_keypress(0x00E4 + UNICODE_BELOW_256, 0);
        input.handle_keys();
        assert!(input.output.reports.len() == 0);
        assert!(input.events.is_empty()); // we eat the keypress though

        input.add_keyrelease(0x00E4 + UNICODE_BELOW_256, 0);
        input.handle_keys();
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
        let mut h = [Box::new(ub) as Box<dyn ProcessKeys<KeyOutCatcher>>];
        let mut input = Input::new(&mut h, KeyOutCatcher::new());
        input.output.set_unicode_mode(UnicodeSendMode::WinCompose);
        //no output on press
        input.add_keypress(0x03B4, 0);
        input.handle_keys();
        assert!(input.output.reports.len() == 0);
        assert!(input.events.is_empty()); // we eat the keypress though

        input.add_keyrelease(0x03B4, 0);
        input.handle_keys();
        check_output(&input, &[&[RAlt], &[U], &[Kb3], &[B], &[Kb4], &[]]);
        assert!(input.events.is_empty()); // we eat the keypress though
    }

    #[test]
    fn test_unicode_while_depressed() {
        use KeyCode::*;
        let mut h = [Box::new(UnicodeKeyboard {}) as Box<dyn ProcessKeys<KeyOutCatcher>>,
                Box::new(USBKeyboard::new())];
        let mut input = Input::new(&mut h, KeyOutCatcher::new());
        input.output.set_unicode_mode(UnicodeSendMode::WinCompose);
        input.add_keypress(A.into(), 0);
        input.handle_keys();
        check_output(&input, &[&[A]]);
        input.output.clear();
        input.add_keypress(0x3B4, 0);
        input.handle_keys();
        check_output(&input, &[&[A]]);
        input.add_keyrelease(0x3B4, 0);
        input.output.clear();
        input.handle_keys();
        check_output(&input, &[
           &[RAlt], &[U], &[Kb3], &[B], &[Kb4], &[],
            &[A]]);
        input.add_keyrelease(A.into(), 0);
        input.output.clear();
        input.handle_keys();
        check_output(&input, &[&[]]);
        assert!(input.events.is_empty());
  


    }
}
