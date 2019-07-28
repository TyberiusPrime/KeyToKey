use no_std_compat::prelude::v1::*;

use crate::key_codes::KeyCode;
use crate::{
    iter_unhandled_mut, Event, EventStatus, Keyboard, KeyboardState, ProcessKeys, USBKeyOut,
};

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
pub fn check_output(keyboard: &Keyboard<KeyOutCatcher>, should: &[&[KeyCode]]) {
    assert!(should.len() == keyboard.output.reports.len());
    for (ii, report) in should.iter().enumerate() {
        assert!(keyboard.output.reports[ii].len() == report.len());
        for k in report.iter() {
            let kcu: u8 = (*k).into();
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
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
        for (event, _status) in iter_unhandled_mut(events) {
            match event {
                Event::TimeOut(ms_since_last) => {
                    if *ms_since_last > self.min_timeout_ms {
                        output.send_keys(&[self.keycode]);
                    }
                }
                _ => {}
            }
        }
    }
}
