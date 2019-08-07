use no_std_compat::prelude::v1::*;
use crate::handlers::{MacroCallback, ProcessKeys};
use crate::key_codes::AcceptsKeycode;
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use crate::USBKeyOut;
pub struct SpaceCadet<M> {
    trigger: u32,
    callbacks: M,
    press_number: u8,
    down: bool,
    activated: bool,
}
impl<M: MacroCallback> SpaceCadet<M> {
    pub fn new(trigger: impl AcceptsKeycode, callbacks: M) -> SpaceCadet<M> {
        SpaceCadet {
            trigger: trigger.to_u32(),
            callbacks,
            press_number: 0, //what was the running id of this?
            down: false,
            activated: false,
        }
    }
}
impl<T: USBKeyOut, M: MacroCallback> ProcessKeys<T> for SpaceCadet<M> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
        let mut initial_keypress_status: Option<EventStatus> = None;
        for (event, status) in iter_unhandled_mut(events) {
            match event {
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.trigger {
                        self.down = false;
                        if kc.running_number == self.press_number + 1 {
                            // a tap
                            //let the downstream handle it!
                            initial_keypress_status = Some(EventStatus::Unhandled);
                        } else {
                            self.callbacks.on_deactivate(output);
                            *status = EventStatus::Handled;
                            initial_keypress_status = Some(EventStatus::Handled);
                        }
                    }
                }
                Event::KeyPress(kc) => {
                    if kc.keycode == self.trigger {
                        *status = EventStatus::Ignored; //skip the scan this time
                        self.press_number = kc.running_number;
                        self.down = true
                    } else if self.down {
                        //trigger has been seen..
                        if !self.activated {
                            self.callbacks.on_activate(output);
                        }
                        self.activated = true;
                        initial_keypress_status = Some(EventStatus::Ignored);
                        //remeber, this is a non-related keypress.
                        //*status = EventStatus::Handled;
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
        match initial_keypress_status {
            Some(new_status) => {
                for (event, status) in events.iter_mut() {
                    match event {
                        Event::KeyPress(kc) => {
                            if kc.keycode == self.trigger {
                                *status = new_status;
                            }
                        }
                        _ => {}
                    }
                }
            }
            None => {}
        }
    }
}
#[cfg(test)]
//#[macro_use]
//extern crate std;
mod tests {
    use crate::handlers::{SpaceCadet, USBKeyboard};
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
    fn test_space_cadet() {
        let counter = Arc::new(RwLock::new(PressCounter {
            down_counter: 0,
            up_counter: 0,
        }));
        let l = SpaceCadet::new(KeyCode::X, counter.clone());
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        //the tap...
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::X, 10);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::X]]);
        keyboard.output.clear();
        assert!(keyboard.events.is_empty());
        //the modifier
        println!("as mod");
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::Z, 0);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[KeyCode::H], &[KeyCode::Z]]);
        assert!(counter.read().down_counter == 1);
        assert!(counter.read().up_counter == 0);
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::Z, 0);
        keyboard.handle_keys().unwrap();
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::X, 10);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::I], &[]]);
        assert!(counter.read().down_counter == 1);
        assert!(counter.read().up_counter == 1);
        keyboard.output.clear();
    }
}
