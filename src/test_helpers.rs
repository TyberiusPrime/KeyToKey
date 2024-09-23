use crate::handlers::{HandlerResult, OnOff, ProcessKeys};
#[allow(unused_imports)]
use crate::key_codes::{AcceptsKeycode, KeyCode};
#[allow(unused_imports)]
use crate::Keyboard;
use crate::{iter_unhandled_mut, Event, EventStatus, KeyboardState, USBKeyOut};
use alloc::sync::Arc;
use no_std_compat::prelude::v1::*;
use spin::RwLock;
#[derive(Default)]
pub struct KeyOutCatcher {
    keys_registered: Vec<u8>,
    pub reports: Vec<Vec<u8>>,
    state: KeyboardState,
    later: Vec<(u32, Vec<KeyCode>)>,
}
impl KeyOutCatcher {
    pub fn new() -> KeyOutCatcher {
        KeyOutCatcher {
            keys_registered: Vec::new(),
            reports: Vec::new(),
            state: KeyboardState::new(),
            later: Vec::new(),
        }
    }
    // for testing, clear the catcher of everything
    pub fn clear(&mut self) {
        self.keys_registered.clear();
        self.reports.clear();
    }
}
impl USBKeyOut for KeyOutCatcher {
    fn state(&mut self) -> &mut KeyboardState {
        return &mut self.state;
    }

    fn ro_state(&self) -> &KeyboardState {
        return &self.state;
    }

    #[allow(unused_variables)]
    fn debug(&mut self, s: &str) {
        #[cfg(test)]
        println!("{}", s);
    }

    fn bootloader(&mut self) {}

    fn send_keys(&mut self, keys: &[KeyCode]) {
        self.reports.push(keys.iter().map(|&x| x.to_u8()).collect());
    }
    fn register_key(&mut self, key: KeyCode) {
        if !self.keys_registered.iter().any(|x| *x == key.to_u8()) {
            self.keys_registered.push(key.to_u8());
        }
    }
    fn send_registered(&mut self) {
        self.reports.push(self.keys_registered.clone());
        self.keys_registered.clear();
    }

    fn send_keys_later(&mut self, _keys: &[KeyCode], _ms: u16) {}
    fn do_send_later(&mut self) {}

    fn send_empty(&mut self) {
        self.reports.push(Vec::new());
    }
}
#[cfg(test)]
pub fn check_output(keyboard: &Keyboard<KeyOutCatcher>, should: &[&[KeyCode]]) {
    if !(should.len() == keyboard.output.reports.len()) {
        dbg!(&keyboard.output.reports);
        dbg!(&should);
    }
    assert!(should.len() == keyboard.output.reports.len());
    for (ii, report) in should.iter().enumerate() {
        if !(keyboard.output.reports[ii].len() == report.len()) {
            dbg!(&keyboard.output.reports);
            dbg!(&should);
        }
        assert!(keyboard.output.reports[ii].len() == report.len());
        for k in report.iter() {
            let kcu: u8 = (*k).to_u8();
            if !(keyboard.output.reports[ii].contains(&kcu)) {
                dbg!(&keyboard.output.reports);
                dbg!(&should);
            }
            assert!(keyboard.output.reports[ii].contains(&kcu));
        }
    }
}
/// send a key whenever a time out occurs
pub struct TimeoutLogger {
    keycode: KeyCode,
    min_timeout_ms: u16,
}
impl TimeoutLogger {
    pub fn new(keycode: KeyCode, min_timeout_ms: u16) -> TimeoutLogger {
        TimeoutLogger {
            keycode,
            min_timeout_ms,
        }
    }
}
impl<T: USBKeyOut> ProcessKeys<T> for TimeoutLogger {
    fn process_keys(
        &mut self,
        events: &mut Vec<(Event, EventStatus)>,
        output: &mut T,
    ) -> HandlerResult {
        for (event, _status) in iter_unhandled_mut(events) {
            if let Event::TimeOut(ms_since_last) = event {
                if *ms_since_last > self.min_timeout_ms {
                    output.send_keys(&[self.keycode]);
                }
            }
        }
        HandlerResult::NoOp
    }
}
#[derive(Debug)]
pub struct PressCounter {
    pub down_counter: u8,
    pub up_counter: u8,
}
impl OnOff for Arc<RwLock<PressCounter>> {
    fn on_activate(&mut self, output: &mut dyn USBKeyOut) {
        self.write().down_counter += 1;
        output.send_keys(&[KeyCode::H]);
    }
    fn on_deactivate(&mut self, output: &mut dyn USBKeyOut) {
        self.write().up_counter += 1;
        output.send_keys(&[KeyCode::I]);
    }
}
impl OnOff for PressCounter {
    fn on_activate(&mut self, output: &mut dyn USBKeyOut) {
        self.down_counter += 1;
        output.send_keys(&[KeyCode::H]);
    }
    fn on_deactivate(&mut self, output: &mut dyn USBKeyOut) {
        self.up_counter += 1;
        output.send_keys(&[KeyCode::I]);
    }
}
#[cfg(test)]
pub struct Debugger {
    s: String,
}
#[cfg(test)]
impl Debugger {
    pub fn new(s: &str) -> Debugger {
        Debugger { s: s.to_string() }
    }
}
#[cfg(test)]
impl<T: USBKeyOut> ProcessKeys<T> for Debugger {
    fn process_keys(
        &mut self,
        events: &mut Vec<(Event, EventStatus)>,
        _output: &mut T,
    ) -> HandlerResult {
        println!("{}, {:?}", self.s, events);
        HandlerResult::NoOp
    }
}

#[cfg(test)]
pub trait Checks {
    /// press check
    fn pc(&mut self, key: impl AcceptsKeycode, should: &[&[KeyCode]]);
    /// release and check
    fn rc(&mut self, key: impl AcceptsKeycode, should: &[&[KeyCode]]);
    /// timeout and check
    fn tc(&mut self, ms_since_last: u16, should: &[&[KeyCode]]);
    ///
    /// press check with defined ms_since
    fn pct(&mut self, key: impl AcceptsKeycode, ms_since_last: u16, should: &[&[KeyCode]]);
    /// release check with defined ms_since
    fn rct(&mut self, key: impl AcceptsKeycode, ms_since_last: u16, should: &[&[KeyCode]]);
}

#[cfg(test)]
impl Checks for Keyboard<'_, KeyOutCatcher> {
    fn pc(&mut self, key: impl AcceptsKeycode, should: &[&[KeyCode]]) {
        self.add_keypress(key, 50);
        self.handle_keys().unwrap();
        check_output(self, should);
        self.output.clear();
    }
    fn rc(&mut self, key: impl AcceptsKeycode, should: &[&[KeyCode]]) {
        self.add_keyrelease(key, 50);
        self.handle_keys().unwrap();
        check_output(self, should);
        self.output.clear();
    }
    fn tc(&mut self, ms_since_last: u16, should: &[&[KeyCode]]) {
        self.add_timeout(ms_since_last);
        self.handle_keys().unwrap();
        check_output(self, should);
        self.output.clear();
    }
    fn pct(&mut self, key: impl AcceptsKeycode, ms_since_last: u16, should: &[&[KeyCode]]) {
        self.add_keypress(key, ms_since_last);
        self.handle_keys().unwrap();
        check_output(self, should);
        self.output.clear();
    }
    fn rct(&mut self, key: impl AcceptsKeycode, ms_since_last: u16, should: &[&[KeyCode]]) {
        self.add_keyrelease(key, ms_since_last);
        self.handle_keys().unwrap();
        check_output(self, should);
        self.output.clear();
    }
}
