use crate::handlers::{ProcessKeys, HandlerResult};
use crate::key_codes::KeyCode;
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use crate::USBKeyOut;
use core::convert::TryInto;
use no_std_compat::prelude::v1::*;

/// Shift keys if they're pressend beyond threshold_ms
/// supposedly for RSI sufferers - this implementation has
/// not been used in daily usage yet.
pub struct AutoShift {
    shift_letters: bool,
    shift_numbers: bool,
    shift_special: bool,
    threshold_ms: u16,
}

impl AutoShift {
    pub fn new(threshold_ms: u16) -> AutoShift {
        AutoShift {
            shift_letters: true,
            shift_numbers: true,
            shift_special: true,
            threshold_ms,
        }
    }
    fn should_autoshift(&self, keycode: u32) -> bool {
        (self.shift_letters && keycode >= KeyCode::A.to_u32() && keycode <= KeyCode::Z.to_u32())
            | (self.shift_numbers
                && keycode >= KeyCode::Kb1.to_u32()
                && keycode <= KeyCode::Kb0.to_u32())
            | (self.shift_special
                && keycode >= KeyCode::Minus.to_u32()
                && keycode <= KeyCode::Slash.to_u32())
    }
}
impl<T: USBKeyOut> ProcessKeys<T> for AutoShift {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> HandlerResult {
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
                                } else {
                                    output.send_keys(&[(kc.keycode as u8).try_into().unwrap()])
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
                if let Event::KeyPress(kc) = event {
                    if handled.contains(&kc.keycode) {
                        *status = EventStatus::Handled;
                    }
                }
            }
        }
    HandlerResult::NoOp
    }
}
#[cfg(test)]
//#[macro_use]
//extern crate std;
mod tests {
    use crate::handlers::{AutoShift, USBKeyboard};
    #[allow(unused_imports)]
    use crate::key_codes::KeyCode;
    #[allow(unused_imports)]
    use crate::test_helpers::{check_output, KeyOutCatcher};
    #[allow(unused_imports)]
    use crate::{
        Event, EventStatus, Keyboard, KeyboardState, ProcessKeys, USBKeyOut, UnicodeSendMode,
    };
    #[allow(unused_imports)]
    use no_std_compat::prelude::v1::*;
    #[test]
    fn test_autoshift() {
        let threshold = 200;
        let l = AutoShift::new(threshold);
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::X, threshold - 1);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::X], &[]]);
        keyboard.output.clear();
        dbg!(&keyboard.events);
        assert!(keyboard.events.is_empty());
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::X, threshold + 1);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[KeyCode::X, KeyCode::LShift], &[]]);
        keyboard.output.clear();
    }
    #[test]
    fn test_autoshift_no_letters() {
        let threshold = 200;
        let mut l = AutoShift::new(threshold);
        l.shift_letters = false;
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        keyboard.add_keypress(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::Kb1, threshold - 1);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::Kb1], &[]]);
        keyboard.output.clear();
        dbg!(&keyboard.events);
        assert!(keyboard.events.is_empty());
        keyboard.add_keypress(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::Kb1, threshold + 1);
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
}
