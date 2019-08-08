use crate::handlers::OnOff;
use crate::key_codes::KeyCode;
use crate::{
    iter_unhandled_mut, Event, EventStatus, Keyboard, KeyboardState, ProcessKeys, USBKeyOut,
};
use alloc::sync::Arc;
use no_std_compat::prelude::v1::*;
use spin::RwLock;
#[derive(Default)]
pub struct KeyOutCatcher {
    keys_registered: Vec<u8>,
    pub reports: Vec<Vec<u8>>,
    state: KeyboardState,
}
impl KeyOutCatcher {
    pub fn new() -> KeyOutCatcher {
        KeyOutCatcher {
            keys_registered: Vec::new(),
            reports: Vec::new(),
            state: KeyboardState::new(),
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
    fn send_empty(&mut self) {
        self.reports.push(Vec::new());
    }
}
pub fn check_output(keyboard: &Keyboard<KeyOutCatcher>, should: &[&[KeyCode]]) {
    assert!(should.len() == keyboard.output.reports.len());
    for (ii, report) in should.iter().enumerate() {
        assert!(keyboard.output.reports[ii].len() == report.len());
        for k in report.iter() {
            let kcu: u8 = (*k).to_u8();
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
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) {
        for (event, _status) in iter_unhandled_mut(events) {
            if let Event::TimeOut(ms_since_last) = event {
                if *ms_since_last > self.min_timeout_ms {
                    output.send_keys(&[self.keycode]);
                }
            }
        }
    }
}
#[derive(Debug)]
pub struct PressCounter {
    pub down_counter: u8,
    pub up_counter: u8,
}
impl OnOff for Arc<RwLock<PressCounter>> {
    fn on_activate(&mut self, output: &mut impl USBKeyOut) {
        self.write().down_counter += 1;
        output.send_keys(&[KeyCode::H]);
    }
    fn on_deactivate(&mut self, output: &mut impl USBKeyOut) {
        self.write().up_counter += 1;
        output.send_keys(&[KeyCode::I]);
    }
}
impl OnOff for PressCounter {
    fn on_activate(&mut self, output: &mut impl USBKeyOut) {
        self.down_counter += 1;
        output.send_keys(&[KeyCode::H]);
    }
    fn on_deactivate(&mut self, output: &mut impl USBKeyOut) {
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
    fn new(s: String) -> Debugger {
        Debugger { s }
    }
}
#[cfg(test)]
impl<T: USBKeyOut> ProcessKeys<T> for Debugger {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, _output: &mut T) -> () {
        println!("{}, {:?}", self.s, events);
    }
}
