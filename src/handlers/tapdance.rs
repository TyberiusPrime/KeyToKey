use no_std_compat::prelude::v1::*;

use crate::handlers::{ProcessKeys};
use crate::USBKeyOut;
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use crate::key_codes::{AcceptsKeycode};

pub struct TapDance<'a, T, F> {
    trigger: u32,
    tap_count: u8,
    on_tap_complete: F, //todo: should we differentiate between timeout and other key by passing an enum?
    //todo: add on_each_tap...
    timeout_ms: u16,
    phantom: core::marker::PhantomData<&'a T>,
}
impl<'a, T: USBKeyOut, F: FnMut(u8, &mut T)> TapDance<'a, T, F> {
    pub fn new(trigger: impl AcceptsKeycode, on_tap_complete: F) -> TapDance<'a, T, F> {
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

#[cfg(test)]
//#[macro_use]
//extern crate std;
mod tests {
    use crate::handlers::{
        TapDance, USBKeyboard
    };
    #[allow(unused_imports)]
    use crate::key_codes::KeyCode;
    #[allow(unused_imports)]
    use crate::test_helpers::{check_output, KeyOutCatcher};
    #[allow(unused_imports)]
    use crate::{
        Event, EventStatus, Keyboard, KeyboardState, ProcessKeys, USBKeyOut, UnicodeSendMode,
    };
    use alloc::sync::Arc;
    #[allow(unused_imports)]
    use no_std_compat::prelude::v1::*;
    use spin::RwLock;
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
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
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



}