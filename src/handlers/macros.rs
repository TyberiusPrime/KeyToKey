use no_std_compat::prelude::v1::*;

use crate::handlers::ProcessKeys;
use crate::USBKeyOut;
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use crate::key_codes::{AcceptsKeycode};
use crate::handlers::MacroCallback;

//// The simples callback -
/// call on_press(output: impl USBKeyOut) on key press
/// and on_release(output) on release))
/// trigger may be any keycode,
/// but preferentialy from the region 0xF00FF..=0xFFFFD
/// which is not used by either UnicodeKeyboard or UsbKeyboard
pub struct PressReleaseMacro<M> {
    keycode: u32,
    callbacks: M,
}
impl<M: MacroCallback> PressReleaseMacro<M> {
    pub fn new(trigger: impl AcceptsKeycode, callbacks: M) -> PressReleaseMacro<M> {
        PressReleaseMacro {
            keycode: trigger.to_u32(),
            callbacks,
        }
    }
}
impl<T: USBKeyOut, M: MacroCallback> ProcessKeys<T> for PressReleaseMacro<M> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
        for (event, status) in iter_unhandled_mut(events){
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
    }
}
/// a macro that is called 'on' on the the first keypress
/// and off on the second keyrelease.
/// Using this you can implement e.g. sticky modifiers
///
pub struct StickyMacro<'a, T, F1: FnMut(&mut T), F2: FnMut(&mut T)> {
    keycode: u32,
    on_toggle_on: F1,
    on_toggle_off: F2,
    active: u8,
    phantom: core::marker::PhantomData<&'a T>,
}
impl<'a, T: USBKeyOut, F1: FnMut(&mut T), F2: FnMut(&mut T)> StickyMacro<'a, T, F1, F2> {
    pub fn new(
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
        for (event, status) in iter_unhandled_mut(events){
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

#[cfg(test)]
//#[macro_use]
//extern crate std;
mod tests {
    use crate::handlers::{
     PressReleaseMacro, StickyMacro, USBKeyboard
    };
    #[allow(unused_imports)]
    use crate::key_codes::KeyCode;
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
        let t = PressReleaseMacro::new(0xF0000u32, counter.clone());
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(t));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        //first press - sets
        keyboard.add_keypress(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(counter.read().down_counter == 1);
        assert!(counter.read().up_counter == 0);
        assert!(keyboard.events.is_empty());
        check_output(&keyboard, &[&[KeyCode::H], &[]]);
        keyboard.output.clear();
        //first release - no change
        keyboard.add_keyrelease(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(counter.read().down_counter == 1);
        assert!(counter.read().up_counter == 1);
        check_output(&keyboard, &[&[KeyCode::I], &[]]);
        keyboard.output.clear();
    }

  #[test]
    fn test_sticky_macro() {
        let l = StickyMacro::new(
            KeyCode::X,
            |output: &mut KeyOutCatcher| output.send_keys(&[KeyCode::A]),
            |output: &mut KeyOutCatcher| output.send_keys(&[KeyCode::B]),
        );
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
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

}