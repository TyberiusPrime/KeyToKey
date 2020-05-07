use crate::handlers::{OnOff, ProcessKeys, Action, HandlerResult};
use crate::key_codes::AcceptsKeycode;
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use crate::USBKeyOut;
use lazy_static::lazy_static;
use no_std_compat::prelude::v1::*;
use spin::RwLock;
#[repr(u8)]
#[derive(Debug)]
pub enum OneShotStatus {
    Held,
    HeldUsed,
    Triggered,
    TriggerUsed,
    Off,
}
/// A OneShot key
/// press it, on_activate will be called,
/// on_deactivate will be called after the next non-oneshot key release
/// or if the OneShot trigger is pressed again
/// 
/// Also, if the OneShot trigger is pressed again on_double_tap_triggerX is called 
/// (after callbacks.on_deactivate, use ActionNone for no action)
///
/// If held_timeout is > 0 and the key is pressed for at least that many ms,
/// and on_deactivate will be called upon release. This typically is useful
/// for graphics work where the user presses the modifiers while interacting
/// with the mouse
///
/// You may also define a released_timeout - after this time, without
/// a different keypress, the OneShot will also deactivate
///
/// OneShots have two triggers to accomodate the usual left/right modifier keys,
/// just pass in Keycode::No if you want one trigger to be ignored
/// note that the oneshots always lead to the left variant of the modifier being sent,
/// even if they're being triggered by the right one.
pub struct OneShot<M1, M2, M3> {
    trigger1: u32,
    trigger2: u32,
    callbacks: M1,
    on_double_tap_trigger1: M2,
    on_double_tap_trigger2: M3,
    status: OneShotStatus,
    held_timeout: u16,
    released_timeout: u16,
}
lazy_static! {
    /// oneshots don't deactive on other oneshots - this stores the keycodes to ignore
    pub static ref ONESHOT_TRIGGERS: RwLock<Vec<u32>> = RwLock::new(Vec::new());
}
impl<M1: OnOff, M2: Action, M3: Action> OneShot<M1, M2, M3> {
    pub fn new(
        trigger1: impl AcceptsKeycode,
        trigger2: impl AcceptsKeycode,
        callbacks: M1,
        on_double_tap_trigger1: M2,
        on_double_tap_trigger2: M3,
        held_timeout: u16,
        released_timeout: u16,
    ) -> OneShot<M1, M2, M3> {
        ONESHOT_TRIGGERS.write().push(trigger1.to_u32());
        ONESHOT_TRIGGERS.write().push(trigger2.to_u32());
        OneShot {
            trigger1: trigger1.to_u32(),
            trigger2: trigger2.to_u32(),
            callbacks,
            on_double_tap_trigger1,
            on_double_tap_trigger2,
            status: OneShotStatus::Off,
            held_timeout,
            released_timeout,
        }
    }
}
impl<T: USBKeyOut, M1: OnOff, M2: Action, M3: Action> ProcessKeys<T> for OneShot<M1, M2, M3> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> HandlerResult {
        for (event, status) in iter_unhandled_mut(events) {
            //a sticky key
            // on press if not active -> active
            // on other key release -> deactivate
            // on press if active -> deactive
            // on release -> noop?
            match event {
                Event::KeyPress(kc) => {
                    if kc.keycode == self.trigger1 || kc.keycode == self.trigger2 {
                        *status = EventStatus::Handled;
                        match self.status {
                            OneShotStatus::Triggered => {
                                self.status = OneShotStatus::Off;
                                self.callbacks.on_deactivate(output);
                                if kc.keycode == self.trigger1 {
                                    self.on_double_tap_trigger1.on_trigger(output);
                                } else if kc.keycode == self.trigger2 {
                                    self.on_double_tap_trigger2.on_trigger(output);
                                }
                            }
                            OneShotStatus::Off => {
                                self.status = OneShotStatus::Held;
                                self.callbacks.on_activate(output)
                            }
                            OneShotStatus::Held
                            | OneShotStatus::HeldUsed
                            | OneShotStatus::TriggerUsed => {}
                        }
                    } else if !ONESHOT_TRIGGERS.read().contains(&kc.keycode) {
                        match self.status {
                            OneShotStatus::Triggered => self.status = OneShotStatus::TriggerUsed,
                            OneShotStatus::TriggerUsed => {
                                self.status = OneShotStatus::Off;
                                self.callbacks.on_deactivate(output)
                            }
                            _ => {}
                        }
                    }
                }
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.trigger1 || kc.keycode == self.trigger2 {
                        match self.status {
                            OneShotStatus::Held => {
                                if self.held_timeout > 0 && kc.ms_since_last > self.held_timeout {
                                    self.status = OneShotStatus::Off;
                                    self.callbacks.on_deactivate(output)
                                } else {
                                    self.status = OneShotStatus::Triggered;
                                }
                            }

                            OneShotStatus::HeldUsed => {
                                self.status = OneShotStatus::Off;
                                self.callbacks.on_deactivate(output)
                            }
                            _ => {}
                        }
                        *status = EventStatus::Handled;
                    } else if !ONESHOT_TRIGGERS.read().contains(&kc.keycode) {
                        match self.status {
                            OneShotStatus::Triggered => {
                                self.status = OneShotStatus::Off;
                                self.callbacks.on_deactivate(output)
                            }
                            OneShotStatus::Held => self.status = OneShotStatus::HeldUsed,
                            _ => {}
                        }
                    }
                }
                Event::TimeOut(ms) => {
                    if let OneShotStatus::Triggered = self.status {
                        if self.released_timeout > 0 && *ms >= self.released_timeout {
                            self.status = OneShotStatus::Off;
                            self.callbacks.on_deactivate(output)
                        }
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
    use crate::handlers::{OneShot, USBKeyboard};
    #[allow(unused_imports)]
    use crate::key_codes::{KeyCode, UserKey};
    #[allow(unused_imports)]
    use crate::test_helpers::{check_output, KeyOutCatcher, PressCounter};
    #[allow(unused_imports)]
    use crate::{
        Event, EventStatus, Keyboard, KeyboardState, ProcessKeys, USBKeyOut, UnicodeSendMode,
    };
    use crate::premade::ActionNone;
    use alloc::sync::Arc;
    #[allow(unused_imports)]
    use no_std_compat::prelude::v1::*;
    use spin::RwLock;
    #[test]
    fn test_oneshot() {
        let counter = Arc::new(RwLock::new(PressCounter {
            down_counter: 0,
            up_counter: 0,
        }));
        let t = OneShot::new(UserKey::UK0, UserKey::UK1, counter.clone(), ActionNone{}, ActionNone{}, 0, 0);
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(t));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        for trigger in [UserKey::UK0, UserKey::UK1].iter() {
            counter.write().down_counter = 0;
            counter.write().up_counter = 0;
            keyboard.output.clear();
            //first press - sets
            keyboard.add_keypress(trigger, 0);
            keyboard.handle_keys().unwrap();
            dbg!(counter.read());
            assert!(counter.read().down_counter == 1);
            assert!(counter.read().up_counter == 0);
            assert!(keyboard.events.is_empty());
            check_output(&keyboard, &[&[KeyCode::H], &[]]);
            keyboard.output.clear();
            //first release - no change
            keyboard.add_keyrelease(trigger, 0);
            keyboard.handle_keys().unwrap();
            assert!(counter.read().down_counter == 1);
            assert!(counter.read().up_counter == 0);
            assert!(keyboard.events.is_empty());
            //second press - unsets
            keyboard.add_keypress(trigger, 0);
            keyboard.handle_keys().unwrap();
            assert!(counter.read().down_counter == 1);
            assert!(counter.read().up_counter == 1);
            assert!(keyboard.events.is_empty());
            //second release - no change
            keyboard.add_keyrelease(trigger, 0);
            keyboard.handle_keys().unwrap();
            assert!(counter.read().down_counter == 1);
            assert!(counter.read().up_counter == 1);
            assert!(keyboard.events.is_empty());
            //third press - sets
            keyboard.add_keypress(trigger, 0);
            keyboard.handle_keys().unwrap();
            assert!(counter.read().down_counter == 2);
            assert!(counter.read().up_counter == 1);
            assert!(keyboard.events.is_empty());
            keyboard.add_keypress(KeyCode::A, 20);
            keyboard.handle_keys().unwrap();
            assert!(counter.read().down_counter == 2);
            assert!(counter.read().up_counter == 1);
            keyboard.add_keyrelease(KeyCode::A, 20);
            keyboard.handle_keys().unwrap();
            assert!(counter.read().down_counter == 2);
            assert!(counter.read().up_counter == 1); //trigger is being held
                                                     //third release - release trigger after other
            keyboard.add_keyrelease(trigger, 0);
            keyboard.handle_keys().unwrap();
            dbg!(&counter);
            assert!(counter.read().down_counter == 2);
            assert!(counter.read().up_counter == 2);
            assert!(keyboard.events.is_empty());
            //fourth press - sets
            keyboard.add_keypress(trigger, 0);
            keyboard.handle_keys().unwrap();
            assert!(counter.read().down_counter == 3);
            assert!(counter.read().up_counter == 2);
            assert!(keyboard.events.is_empty());
            //fifth release - no change
            keyboard.add_keyrelease(trigger, 0);
            keyboard.handle_keys().unwrap();
            assert!(counter.read().down_counter == 3);
            assert!(counter.read().up_counter == 2);
            assert!(keyboard.events.is_empty());
            //sixth press - up
            keyboard.add_keypress(trigger, 0);
            keyboard.handle_keys().unwrap();
            assert!(counter.read().down_counter == 3);
            assert!(counter.read().up_counter == 3);
            assert!(keyboard.events.is_empty());
            //sixth release - no change
            keyboard.add_keyrelease(trigger, 0);
            keyboard.handle_keys().unwrap();
            assert!(counter.read().down_counter == 3);
            assert!(counter.read().up_counter == 3);
            assert!(keyboard.events.is_empty());
        }
        //what happens if you use both triggers
        counter.write().down_counter = 0;
        counter.write().up_counter = 0;
        keyboard.output.clear();
        //first press - sets
        keyboard.add_keypress(UserKey::UK0, 0);
        keyboard.handle_keys().unwrap();
        dbg!(counter.read());
        assert!(counter.read().down_counter == 1);
        assert!(counter.read().up_counter == 0);
        assert!(keyboard.events.is_empty());
        check_output(&keyboard, &[&[KeyCode::H], &[]]);
        keyboard.output.clear();
        //first release - no change
        keyboard.add_keyrelease(UserKey::UK0, 0);
        keyboard.handle_keys().unwrap();
        assert!(counter.read().down_counter == 1);
        assert!(counter.read().up_counter == 0);
        assert!(keyboard.events.is_empty());
        //second press - unsets
        keyboard.add_keypress(UserKey::UK1, 0);
        keyboard.handle_keys().unwrap();
        assert!(counter.read().down_counter == 1);
        assert!(counter.read().up_counter == 1);
        assert!(keyboard.events.is_empty());
        //second release - no change
        keyboard.add_keyrelease(UserKey::UK1, 0);
        keyboard.handle_keys().unwrap();
        assert!(counter.read().down_counter == 1);
        assert!(counter.read().up_counter == 1);
        assert!(keyboard.events.is_empty());
        //third press - sets
        keyboard.add_keypress(UserKey::UK1, 0);
        keyboard.handle_keys().unwrap();
        assert!(counter.read().down_counter == 2);
        assert!(counter.read().up_counter == 1);
        assert!(keyboard.events.is_empty());
        keyboard.add_keypress(KeyCode::A, 20);
        keyboard.handle_keys().unwrap();
        assert!(counter.read().down_counter == 2);
        assert!(counter.read().up_counter == 1);
        keyboard.add_keyrelease(KeyCode::A, 20);
        keyboard.handle_keys().unwrap();
        assert!(counter.read().down_counter == 2);
        assert!(counter.read().up_counter == 1); // still being held
                                                 //third release - triggers deactivate
        keyboard.add_keyrelease(UserKey::UK1, 0);
        keyboard.handle_keys().unwrap();
        assert!(counter.read().down_counter == 2);
        assert!(counter.read().up_counter == 2);
        assert!(keyboard.events.is_empty());
        //fourth press - sets
        keyboard.add_keypress(UserKey::UK0, 0);
        keyboard.handle_keys().unwrap();
        assert!(counter.read().down_counter == 3);
        assert!(counter.read().up_counter == 2);
        assert!(keyboard.events.is_empty());
        //fifth release - no change
        keyboard.add_keyrelease(UserKey::UK0, 0);
        keyboard.handle_keys().unwrap();
        assert!(counter.read().down_counter == 3);
        assert!(counter.read().up_counter == 2);
        assert!(keyboard.events.is_empty());
        //sixth press - up
        keyboard.add_keypress(UserKey::UK1, 0);
        keyboard.handle_keys().unwrap();
        assert!(counter.read().down_counter == 3);
        assert!(counter.read().up_counter == 3);
        assert!(keyboard.events.is_empty());
        //sixth release - no change
        keyboard.add_keyrelease(UserKey::UK1, 0);
        keyboard.handle_keys().unwrap();
        assert!(counter.read().down_counter == 3);
        assert!(counter.read().up_counter == 3);
        assert!(keyboard.events.is_empty());
    }
    #[test]
    fn test_oneshot_timeout() {
        let counter = Arc::new(RwLock::new(PressCounter {
            down_counter: 0,
            up_counter: 0,
        }));
        let timeout = 1000;
        let t = OneShot::new(UserKey::UK0, UserKey::UK1, counter.clone(), ActionNone{}, ActionNone{}, timeout, 0);
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(t));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        for trigger in [UserKey::UK0, UserKey::UK1].iter() {
            counter.write().down_counter = 0;
            counter.write().up_counter = 0;
            keyboard.output.clear();
            //first press - sets
            keyboard.add_keypress(trigger, 0);
            keyboard.handle_keys().unwrap();
            dbg!(counter.read());
            assert!(counter.read().down_counter == 1);
            assert!(counter.read().up_counter == 0);
            assert!(keyboard.events.is_empty());
            check_output(&keyboard, &[&[KeyCode::H], &[]]);
            keyboard.output.clear();
            //first release - no change
            keyboard.add_keyrelease(trigger, timeout + 1);
            keyboard.handle_keys().unwrap();
            assert!(counter.read().down_counter == 1);
            assert!(counter.read().up_counter == 1);
            assert!(keyboard.events.is_empty());
        }
    }

    #[test]
    fn test_oneshot_double_tap() {
        use crate::key_codes::KeyCode::*;
        use crate::handlers::Action;
        use crate::test_helpers::Checks;
        let counter = Arc::new(RwLock::new(PressCounter {
            down_counter: 0,
            up_counter: 0,
        }));
        let timeout = 1000;
        struct MyAction {
            keycode: KeyCode
        }
        impl Action for MyAction { 
            fn on_trigger(&mut self, output: &mut dyn USBKeyOut) {
                output.send_keys(&[self.keycode]);
            }
        }
        let t = OneShot::new(UserKey::UK0, UserKey::UK1, counter.clone(), MyAction{keycode: A}, MyAction{keycode:B}, timeout, 0);
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(t));
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        let trigger = UserKey::UK0;
        keyboard.pc(trigger, &[&[H], &[]]);
        keyboard.rc(trigger, &[&[]]);
        keyboard.pc(trigger, &[&[I], &[A], &[]]);
        keyboard.rc(trigger, &[&[]]);

        assert!(counter.read().down_counter == 1);
        assert!(counter.read().up_counter == 1);

        let trigger = UserKey::UK1;
        keyboard.pc(trigger, &[&[H], &[]]);
        keyboard.rc(trigger, &[&[]]);
        keyboard.pc(trigger, &[&[I], &[B], &[]]);
        keyboard.rc(trigger, &[&[]]);

        assert!(counter.read().down_counter == 2);
        assert!(counter.read().up_counter == 2);
    }

}
