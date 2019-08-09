use crate::handlers::Action;
use crate::handlers::ProcessKeys;
use crate::key_codes::AcceptsKeycode;
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use crate::USBKeyOut;
use no_std_compat::prelude::v1::*;

pub struct LongTap<M1, M2> {
    trigger: u32,
    action_short: M1,
    action_long: M2,
    threshold_ms: u16,
}

impl<M1: Action, M2: Action> LongTap<M1, M2> {
    pub fn new(
        trigger: impl AcceptsKeycode,
        action_short: M1,
        action_long: M2,
        threshold_ms: u16,
    ) -> LongTap<M1, M2> {
        LongTap {
            trigger: trigger.to_u32(),
            action_short,
            action_long,
            threshold_ms,
        }
    }
}

/// Handle that does one thing on a short tab,
/// and another on a long tab with a configurable threshold
///
/// Action happens on release
///
/// Note that this is a simple implementation that just considers
/// the time from the last key-event withouth considering
/// whether that was actually the press of the LongTap key
impl<T: USBKeyOut, M1: Action, M2: Action> ProcessKeys<T> for LongTap<M1, M2> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) {
        for (event, status) in iter_unhandled_mut(events).rev() {
            match event {
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.trigger {
                        *status = EventStatus::Handled;
                        if kc.ms_since_last >= self.threshold_ms {
                            self.action_long.on_trigger(output);
                        } else {
                            self.action_short.on_trigger(output)
                        }
                    }
                }
                Event::KeyPress(kc) => {
                    if kc.keycode == self.trigger {
                        *status = EventStatus::Handled;
                    }
                }

                _ => {}
            }
        }
    }
}

#[cfg(test)]
//#[macro_use]
//extern crate std;
mod tests {
    use crate::handlers::{Action, LongTap, USBKeyboard};
    #[allow(unused_imports)]
    use crate::key_codes::{KeyCode, UserKey};
    #[allow(unused_imports)]
    use crate::test_helpers::{check_output, KeyOutCatcher};
    #[allow(unused_imports)]
    use crate::{
        Event, EventStatus, Keyboard, KeyboardState, ProcessKeys, USBKeyOut, UnicodeSendMode,
    };
    #[allow(unused_imports)]
    use no_std_compat::prelude::v1::*;

    #[test]
    fn test_longtab() {
        struct Short();
        struct Long();
        impl Action for Short {
            fn on_trigger(&mut self, output: &mut impl USBKeyOut) {
                output.send_keys(&[KeyCode::A]);
            }
        }
        impl Action for Long {
            fn on_trigger(&mut self, output: &mut impl USBKeyOut) {
                output.send_keys(&[KeyCode::B]);
            }
        }

        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let timeout: u16 = 1000;
        keyboard.add_handler(Box::new(LongTap::new(
            UserKey::UK0,
            Short {},
            Long {},
            timeout,
        )));
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        keyboard.add_keypress(UserKey::UK0, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(UserKey::UK0, timeout - 1);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[KeyCode::A], &[]]);
        keyboard.output.clear();

        keyboard.add_keypress(UserKey::UK0, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(UserKey::UK0, timeout);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[KeyCode::B], &[]]);
        keyboard.output.clear();
    }

    #[test]
    fn test_longtab_plus_mod() {
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let timeout: u16 = 1000;
        keyboard.add_handler(Box::new(LongTap::new(
            UserKey::UK0,
            KeyCode::A,
            KeyCode::B,
            timeout,
        )));
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        keyboard.add_keypress(UserKey::UK0, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(UserKey::UK0, timeout - 1);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[KeyCode::A]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::LShift, 0);
        keyboard.handle_keys().unwrap();
        keyboard.output.clear();
        keyboard.add_keypress(UserKey::UK0, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(UserKey::UK0, timeout);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[KeyCode::B, KeyCode::LShift]]);
        keyboard.output.clear();
    }
}
