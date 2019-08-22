use crate::handlers::{Action, OnOff, ProcessKeys, HandlerResult};
use crate::key_codes::AcceptsKeycode;
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use crate::USBKeyOut;
use no_std_compat::prelude::v1::*;

#[repr(u8)]
#[derive(Clone, Copy)]
enum SpaceCadetState {
    Base,       //not triggrered
    Pressed,    //could be either a tap or an onoff
    Activated,  //an onoff
    PressedTap, //must be a tap
}

/// SpaceCadet Keys
/// are keys that do one Action on tap,
/// and an OnOff if depressed while the
/// next key is pressed.
///
/// There is a minimum time the key needs to be depressed,
/// which you can configure with SpaceCadet.minimum_depress_ms
/// (this is to allow fast typing where you actually hit the next
/// key before the previous one has been released. It does
/// happend...)
///
/// They need to be added before
/// the layer they toggle (if used with a layer),
/// so you will have to use keyboard.future_handler_id(2)
/// for the handler id in premade::spacecadet_handler
///
/// Please note if you want a premade::one_shot_* (modifier) to
/// work correctly with a space cadet,
/// the one_shot must come first in the list of handlers
/// otherwise it will only work like a regular modifier with the space
/// cadet trigger.
pub struct SpaceCadet<MAction, MOnOff> {
    trigger: u32,
    action: MAction,
    onoff: MOnOff,
    press_number: u8,
    state: SpaceCadetState,
    pub minimum_depress_ms: u16,
}
impl<MAction: Action, MOnOff: OnOff> SpaceCadet<MAction, MOnOff> {
    pub fn new(
        trigger: impl AcceptsKeycode,
        action: MAction,
        onoff: MOnOff,
    ) -> SpaceCadet<MAction, MOnOff> {
        SpaceCadet {
            trigger: trigger.to_u32(),
            action,
            onoff,
            press_number: 0, //what was the running id of this?
            state: SpaceCadetState::Base,
            minimum_depress_ms: 100,
        }
    }
}
impl<T: USBKeyOut, MAction: Action, MOnOff: OnOff> ProcessKeys<T> for SpaceCadet<MAction, MOnOff> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) ->HandlerResult {
        let mut any_other_seen = false;
        for (event, status) in iter_unhandled_mut(events) {
            match event {
                Event::KeyPress(kc) => {
                    if kc.keycode == self.trigger {
                        if kc.flag & 0x1 == 0 {
                            //the flag is necessary to prevent rewritten keys from triggering again
                            if any_other_seen {
                                self.state = SpaceCadetState::PressedTap;
                                self.action.on_trigger(output);
                                self.state = SpaceCadetState::Base;
                            } else {
                                self.state = SpaceCadetState::Pressed;
                            }
                        }
                        *status = EventStatus::Handled;
                    } else {
                        match self.state {
                            SpaceCadetState::Pressed => {
                                if kc.ms_since_last >= self.minimum_depress_ms {
                                    self.state = SpaceCadetState::Activated;
                                    self.onoff.on_activate(output);
                                } else {
                                    //a 'botched' activation
                                    self.action.on_trigger(output);
                                    self.state = SpaceCadetState::Base;
                                }
                            }
                            SpaceCadetState::Base => {
                                any_other_seen = true;
                            }
                            _ => {}
                        }
                    }
                }
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.trigger {
                        match self.state {
                            SpaceCadetState::Pressed => {
                                self.action.on_trigger(output);
                                self.state = SpaceCadetState::Base;
                            }
                            SpaceCadetState::Activated => {
                                self.state = SpaceCadetState::Base;
                                self.onoff.on_deactivate(output);
                            }
                            SpaceCadetState::Base | SpaceCadetState::PressedTap => {}
                        }
                    }
                }
                _ => {}
            }
        }
    HandlerResult::NoOp
    }
}

/*
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
                    self.activated = false;
                    *status = EventStatus::Handled;
                    initial_keypress_status = Some(EventStatus::Handled);
                }
            }
        }
        Event::KeyPress(kc) => {
            if kc.keycode == self.trigger && !self.activated {
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
if let Some(new_status) = initial_keypress_status {
    for (event, status) in events.iter_mut() {
        if let Event::KeyPress(kc) = event {
            if kc.running_number == self.press_number {
                *status = new_status;
            }
        }
    }
}
*/
*/

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
        let l = SpaceCadet::new(KeyCode::X, KeyCode::X, counter.clone());
        let threshold = l.minimum_depress_ms;
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
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::Z, threshold);
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

    fn test_space_cadet_fast_typing() {
        let counter = Arc::new(RwLock::new(PressCounter {
            down_counter: 0,
            up_counter: 0,
        }));
        let l = SpaceCadet::new(KeyCode::X, KeyCode::X, counter.clone());
        let threshold = l.minimum_depress_ms;
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        //too fast
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::Z, threshold - 1);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[KeyCode::Z]]);
        assert!(counter.read().down_counter == 0);
        assert!(counter.read().up_counter == 0);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::Z, 0);
        keyboard.handle_keys().unwrap();
        keyboard.output.clear();
        check_output(&keyboard, &[&[]]);

        //now even though we're now slow enough, we don't activate anymore

        keyboard.add_keypress(KeyCode::A, threshold);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[KeyCode::A]]);
        assert!(counter.read().down_counter == 0);
        assert!(counter.read().up_counter == 0);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 10);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        assert!(counter.read().down_counter == 0);
        assert!(counter.read().up_counter == 0);
        keyboard.output.clear();
    }

    #[test]
    fn test_space_cadet_layer() {
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        use crate::handlers::LayerAction::RewriteTo as RT;
        let l = crate::premade::space_cadet_handler(
            KeyCode::X,
            KeyCode::U,
            keyboard.future_handler_id(2),
        );
        let threshold = l.minimum_depress_ms;
        keyboard.add_handler(l);

        let numpad_id = keyboard.add_handler(Box::new(crate::handlers::Layer::new(vec![
            (KeyCode::U, RT(KeyCode::Kb7.into())),
            (KeyCode::I, RT(KeyCode::Kb8.into())),
            (KeyCode::O, RT(KeyCode::Kb9.into())),
            (KeyCode::J, RT(KeyCode::Kb4.into())),
            (KeyCode::K, RT(KeyCode::Kb5.into())),
            (KeyCode::L, RT(KeyCode::Kb6.into())),
            (KeyCode::M, RT(KeyCode::Kb1.into())),
            (KeyCode::Comma, RT(KeyCode::Kb2.into())),
            (KeyCode::Dot, RT(KeyCode::Kb3.into())),
            (KeyCode::Up, RT(KeyCode::Kb0.into())),
            (KeyCode::Space, RT(KeyCode::Tab.into())),
            (KeyCode::Down, RT(KeyCode::Dot.into())),
            (KeyCode::LBracket, RT(KeyCode::Comma.into())),
        ],crate::handlers::AutoOff::No
        )));
        keyboard.output.state().disable_handler(numpad_id);
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        //the modifier
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::U, threshold);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        assert!(keyboard.output.state().is_handler_enabled(numpad_id));
        check_output(&keyboard, &[&[KeyCode::Kb7]]);
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::U, 10);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        assert!(!keyboard.output.state().is_handler_enabled(numpad_id));
        keyboard.output.clear();
    }

    #[test]
    fn test_space_cadet_oneshot_interaction() {
        use crate::premade;
        use crate::test_helpers::Checks;
        use crate::Modifier::Shift;

        let counter = Arc::new(RwLock::new(PressCounter {
            down_counter: 0,
            up_counter: 0,
        }));
        let l = SpaceCadet::new(KeyCode::X, KeyCode::X, counter.clone());
        let mut k = Keyboard::new(KeyOutCatcher::new());
        k.add_handler(premade::one_shot_shift(400, 1000));
        k.add_handler(Box::new(l));
        k.add_handler(Box::new(USBKeyboard::new()));

        k.pc(KeyCode::RShift, &[&[KeyCode::LShift]]); // remember, one shots always output the left.
        k.rc(KeyCode::RShift, &[&[KeyCode::LShift]]); //shift stays pressed by the one shot.

        assert!(k.output.state().modifier(Shift));
        k.pc(KeyCode::X, &[&[KeyCode::LShift]]);
        k.rc(KeyCode::X, &[&[KeyCode::LShift, KeyCode::X]]);
    }

    /*
        #[test]
        fn test_space_cadet_rewrite() {
            use crate::premade::dvorak;
            use crate::test_helpers::Debugger;
            let counter = Arc::new(RwLock::new(PressCounter {
                down_counter: 0,
                up_counter: 0,
            }));
            let l = SpaceCadet::new(KeyCode::F, KeyCode::F, counter.clone());
            let mut keyboard = Keyboard::new(KeyOutCatcher::new());
            keyboard.add_handler(Box::new(l));
            //keyboard.add_handler(Box::new(Debugger::new("cadet".to_string())));
            let dv = keyboard.add_handler(dvorak());
            //keyboard.add_handler(Box::new(Debugger::new("dv".to_string())));
            keyboard.output.state().enable_handler(dv);
            keyboard.add_handler(Box::new(USBKeyboard::new()));
            //the tap...
            println!("adding f");
            keyboard.add_keypress(KeyCode::F, 0);
            keyboard.handle_keys().unwrap();
            check_output(&keyboard, &[&[]]);
            keyboard.output.clear();
            println!("adding F");
            keyboard.add_keyrelease(KeyCode::F, 10);
            keyboard.handle_keys().unwrap();
            check_output(&keyboard, &[&[KeyCode::U]]);
            keyboard.output.clear();
            assert!(keyboard.events.is_empty());

            //the modifier
            keyboard.add_keypress(KeyCode::F, 0);
            keyboard.handle_keys().unwrap();
            check_output(&keyboard, &[&[]]);
            keyboard.output.clear();
            keyboard.add_keypress(KeyCode::C, 0);
            keyboard.handle_keys().unwrap();
            dbg!(&keyboard.output.reports);
            check_output(&keyboard, &[&[KeyCode::H], &[KeyCode::J]]);
            assert!(counter.read().down_counter == 1);
            assert!(counter.read().up_counter == 0);
            keyboard.output.clear();

            keyboard.add_keyrelease(KeyCode::F, 10);
            keyboard.handle_keys().unwrap();
            check_output(&keyboard, &[&[KeyCode::I], &[KeyCode::J]]);
            assert!(counter.read().down_counter == 1);
            assert!(counter.read().up_counter == 1);
            keyboard.output.clear();

            keyboard.add_keyrelease(KeyCode::C, 0);
            keyboard.handle_keys().unwrap();
            check_output(&keyboard, &[&[]]);
            keyboard.output.clear();

            //the modifier - a second time
            keyboard.add_keypress(KeyCode::F, 0);
            keyboard.handle_keys().unwrap();
            check_output(&keyboard, &[&[]]);
            keyboard.output.clear();
            keyboard.add_keypress(KeyCode::C, 0);
            keyboard.handle_keys().unwrap();
            dbg!(&keyboard.output.reports);
            check_output(&keyboard, &[&[KeyCode::H], &[KeyCode::J]]);
            assert!(counter.read().down_counter == 2);
            assert!(counter.read().up_counter == 1);
            keyboard.output.clear();

            keyboard.add_keyrelease(KeyCode::F, 10);
            keyboard.handle_keys().unwrap();
            check_output(&keyboard, &[&[KeyCode::I], &[KeyCode::J]]);
            assert!(counter.read().down_counter == 2);
            assert!(counter.read().up_counter == 2);
            keyboard.output.clear();

            keyboard.add_keyrelease(KeyCode::C, 0);
            keyboard.handle_keys().unwrap();
            check_output(&keyboard, &[&[]]);
            keyboard.output.clear();



        }

    */
}
