use crate::handlers::{Action, OnOff};
use crate::handlers::{ProcessKeys, HandlerResult};
use crate::key_codes::AcceptsKeycode;
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use crate::USBKeyOut;
use no_std_compat::prelude::v1::*;

/// The simplest callback -
/// call on_trigger on press, nothing on release
/// trigger may be any keycode,
/// but consider using the constants in UserKey::*
/// which is not used by either UnicodeKeyboard or UsbKeyboard
pub struct PressMacro<M> {
    keycode: u32,
    callback: M,
}
impl<M: Action> PressMacro<M> {
    pub fn new(trigger: impl AcceptsKeycode, callback: M) -> PressMacro<M> {
        PressMacro {
            keycode: trigger.to_u32(),
            callback,
        }
    }
}
impl<T: USBKeyOut, M: Action> ProcessKeys<T> for PressMacro<M> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> HandlerResult {
        for (event, status) in iter_unhandled_mut(events) {
            match event {
                Event::KeyPress(kc) => {
                    if kc.keycode == self.keycode {
                        *status = EventStatus::Handled;
                        self.callback.on_trigger(output);
                    }
                }
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.keycode {
                        *status = EventStatus::Handled;
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
    HandlerResult::NoOp
    }
}




/// A simple callback -
/// call on_press(output: impl USBKeyOut) on key press
/// and on_release(output) on release))
/// trigger may be any keycode,
/// but consider using the constants in UserKey::*
/// which is not used by either UnicodeKeyboard or UsbKeyboard
pub struct PressReleaseMacro<M> {
    keycode: u32,
    callbacks: M,
}
impl<M: OnOff> PressReleaseMacro<M> {
    pub fn new(trigger: impl AcceptsKeycode, callbacks: M) -> PressReleaseMacro<M> {
        PressReleaseMacro {
            keycode: trigger.to_u32(),
            callbacks,
        }
    }
}
impl<T: USBKeyOut, M: OnOff> ProcessKeys<T> for PressReleaseMacro<M> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> HandlerResult {
        for (event, status) in iter_unhandled_mut(events) {
            match event {
                Event::KeyPress(kc) => {
                    if kc.keycode == self.keycode {
                        *status = EventStatus::Handled;
                        self.callbacks.on_activate(output);
                    }
                }
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.keycode {
                        *status = EventStatus::Handled;
                        self.callbacks.on_deactivate(output);
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
    HandlerResult::NoOp
    }
}

/// a macro that is called 'on' on the the first keypress
/// and off on the second keyrelease.
/// Using this you can implement e.g. sticky modifiers
///
pub struct StickyMacro<M> {
    keycode: u32,
    callbacks: M,
    active: u8,
}

impl<M: OnOff> StickyMacro<M> {
    pub fn new(trigger: impl AcceptsKeycode, callbacks: M) -> StickyMacro<M> {
        StickyMacro {
            keycode: trigger.to_u32(),
            callbacks,
            active: 0,
        }
    }
}

impl<T: USBKeyOut, M: OnOff> ProcessKeys<T> for StickyMacro<M> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) ->HandlerResult {
        for (event, status) in iter_unhandled_mut(events) {
            //a sticky key
            // on press if not active -> active
            // on 2nd release if active -> deactivate
            match event {
                Event::KeyPress(kc) => {
                    if kc.keycode == self.keycode {
                        if self.active == 0 {
                            self.active = 1;
                            self.callbacks.on_activate(output);
                        } else {
                            self.active = 2;
                        }
                        *status = EventStatus::Handled;
                    }
                }
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.keycode {
                        if self.active == 2 {
                            self.callbacks.on_deactivate(output);
                        }
                        *status = EventStatus::Handled;
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
    HandlerResult::NoOp
    }
}
#[cfg(test)]
//#[macro_use]
//extern crate std;
mod tests {
    use crate::handlers::{PressReleaseMacro, StickyMacro, USBKeyboard};
    #[allow(unused_imports)]
    use crate::key_codes::{KeyCode, UserKey};
    #[allow(unused_imports)]
    use crate::test_helpers::{check_output, KeyOutCatcher, PressCounter};
    #[allow(unused_imports)]
    use crate::{
        Event, EventStatus, Keyboard, KeyboardState, ProcessKeys, USBKeyOut, UnicodeSendMode,
    };
    use alloc::sync::Arc;
    #[allow(unused_imports)]
    use no_std_compat::prelude::v1::*;
    use spin::RwLock;
    #[test]
    fn test_press_release() {
        let counter = Arc::new(RwLock::new(PressCounter {
            down_counter: 0,
            up_counter: 0,
        }));
        let t = PressReleaseMacro::new(UserKey::UK0, counter.clone());
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(t));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        //first press - sets
        keyboard.add_keypress(UserKey::UK0, 0);
        keyboard.handle_keys().unwrap();
        assert!(counter.read().down_counter == 1);
        assert!(counter.read().up_counter == 0);
        assert!(keyboard.events.is_empty());
        check_output(&keyboard, &[&[KeyCode::H], &[]]);
        keyboard.output.clear();
        //first release - no change
        keyboard.add_keyrelease(UserKey::UK0, 0);
        keyboard.handle_keys().unwrap();
        assert!(counter.read().down_counter == 1);
        assert!(counter.read().up_counter == 1);
        check_output(&keyboard, &[&[KeyCode::I], &[]]);
        keyboard.output.clear();
    }

    #[test]
    fn test_sticky_macro() {
        let counter = Arc::new(RwLock::new(PressCounter {
            down_counter: 0,
            up_counter: 0,
        }));
        let l = StickyMacro::new(KeyCode::X, counter.clone());
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        //activate
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::H], &[]]);
        keyboard.output.clear();

        assert!(counter.read().down_counter == 1);
        assert!(counter.read().up_counter == 0);
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

        assert!(counter.read().down_counter == 1);
        assert!(counter.read().up_counter == 0);
        //deactivate
        keyboard.add_keyrelease(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::I], &[]]);
        assert!(counter.read().down_counter == 1);
        assert!(counter.read().up_counter == 1);
    }
}
