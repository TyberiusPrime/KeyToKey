use crate::handlers::{ProcessKeys, HandlerResult};
use crate::key_codes::{AcceptsKeycode, KeyCode};
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use crate::Modifier::*;
use crate::USBKeyOut;
use crate::handlers::oneshot::ONESHOT_TRIGGERS;

use no_std_compat::prelude::v1::*;
pub enum LayerAction<'a> {
    RewriteTo(u32),
    RewriteToShifted(u32, u32),
    //todo: rewrite shift
    SendString(&'a str),
    SendStringShifted(&'a str, &'a str),
    //    Callback(fn(&mut T) -> (), fn(&mut T) -> ()),
}

#[repr(u8)]
pub enum AutoOff {
    No,
    AfterMatch, 
    AfterNonModifier,
    AfterAll
}

/// A layer either rewrites a key to another one
/// or outputs a string upon key release.
///
/// It does this for multiple mappings at once,
/// and it can consider the shift state, which
/// is very useful for unicode characters with lower
/// and upper case.
///
/// Unfortunatly, Layers are memory inefficient,
/// they keep their mapping in ram, and each mapping is at least
/// 96 bits / 12 bytes.
///
/// Consider using a RewriteLayer instead if you don't need
/// the string or Shift functionality.
///
/// If AutoOff is set to anything but AutoOff::No, the layer will turn itself of
/// after any key release (AutoOff::AfterAll), after a non-modifier-non-oneshot
/// key release (AutoOff::AfterNonModifier), or after a successfull 
/// match AutoOff::AfterMatch
pub struct Layer<'a> {
    rewrites: Vec<(u32, LayerAction<'a>)>,
    auto_off: AutoOff
}
impl Layer<'_> {
    pub fn new<F: AcceptsKeycode>(rewrites: Vec<(F, LayerAction)>, 
    auto_off: AutoOff) -> Layer<'_> {
        Layer {
            rewrites: rewrites
                .into_iter()
                .map(|(trigger, action)| (trigger.to_u32(), action))
                .collect(),
            auto_off
        }
    }
}
impl<T: USBKeyOut> ProcessKeys<T> for Layer<'_> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> HandlerResult {
        let mut result = HandlerResult::NoOp;
        for (event, status) in iter_unhandled_mut(events) {
            //events.iter_mut() {
            match event {
                Event::KeyRelease(kc) => {
                    let mut rewrite_happend = false;
                    for (from, to) in self.rewrites.iter() {
                        if *from == kc.keycode {
                            match to {
                                LayerAction::RewriteTo(to_keycode) => {
                                    if (kc.flag & 2) == 0 {
                                        kc.keycode = *to_keycode;
                                        kc.flag |= 2;
                                        rewrite_happend = true;
                                    }
                                    break; //only one rewrite per layer
                                }
                                LayerAction::RewriteToShifted(to_keycode, to_shifted_keycode) => {
                                    if (kc.flag & 2) == 0 {
                                        if output.state().modifier(Shift) {
                                            kc.keycode = *to_shifted_keycode;
                                        } else {
                                            kc.keycode = *to_keycode;
                                        }
                                        kc.flag |= 2;
                                        rewrite_happend = true;
                                    }
                                    break; //only one rewrite per layer
                                }
                                LayerAction::SendString(s) => {
                                    output.send_string(s);
                                    *status = EventStatus::Handled;
                                    rewrite_happend = true;
                                    break; //only one rewrite per layer
                                }
                                LayerAction::SendStringShifted(s1, s2) => {
                                    if output.state().modifier(Shift) {
                                        output.send_string(s2);
                                    } else {
                                        output.send_string(s1);
                                    }
                                    *status = EventStatus::Handled;
                                    rewrite_happend = true;
                                    break; //only one rewrite per layer
                                }
                            }
                        }
                    }
                    let turn_off = match self.auto_off {
                        AutoOff::No => false,
                        AutoOff::AfterAll => true,
                        AutoOff::AfterMatch => rewrite_happend,
                        AutoOff::AfterNonModifier => {
                            !ONESHOT_TRIGGERS.read().contains(&kc.keycode) && ! 
                            ( KeyCode::LCtrl.to_u32() <= kc.keycode && kc.keycode <= KeyCode::RGui.to_u32())
                        }
                    };
                    if turn_off {
                        result = HandlerResult::Disable;
                    }


                }
                Event::KeyPress(kc) => {
                    for (from, to) in self.rewrites.iter() {
                        if *from == kc.keycode {
                            match to {
                                LayerAction::RewriteTo(to_keycode) => {
                                    if (kc.flag & 2) == 0 {
                                        kc.keycode = *to_keycode;
                                        kc.flag |= 2;
                                    }
                                    break; //only one rewrite per layer
                                }
                                LayerAction::RewriteToShifted(to_keycode, to_shifted_keycode) => {
                                    if (kc.flag & 2) == 0 {
                                        if output.state().modifier(Shift) {
                                            kc.keycode = *to_shifted_keycode;
                                        } else {
                                            kc.keycode = *to_keycode;
                                        }
                                        kc.flag |= 2;
                                    }
                                    break; //only one rewrite per layer
                                }
                                LayerAction::SendString(_)
                                | LayerAction::SendStringShifted(_, _) => {
                                    *status = EventStatus::Handled;
                                    break;
                                }
                            }
                        }
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
        result
    }
    fn default_enabled(&self) -> bool {
        false
    }
}
#[cfg(test)]
//#[macro_use]
//extern crate std;
mod tests {
    use crate::handlers::{Layer, LayerAction, USBKeyboard, UnicodeKeyboard, AutoOff};
    #[allow(unused_imports)]
    use crate::key_codes::KeyCode;
    #[allow(unused_imports)]
    use crate::test_helpers::{check_output, KeyOutCatcher};
    use crate::Modifier::*;
    #[allow(unused_imports)]
    use crate::{
        Event, EventStatus, Keyboard, KeyboardState, ProcessKeys, USBKeyOut, UnicodeSendMode,
    };
    #[allow(unused_imports)]
    use no_std_compat::prelude::v1::*;
    #[test]
    fn test_layer_rewrite() {
        let l = Layer::new(vec![(
            KeyCode::A,
            LayerAction::RewriteTo(KeyCode::X.into()),)],
            AutoOff::No,
        );
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let layer_id = keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        keyboard.output.state().enable_handler(layer_id);
        keyboard.add_keypress(KeyCode::B, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::B, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(
            &keyboard,
            &[&[KeyCode::B], &[], &[KeyCode::X], &[], &[KeyCode::X], &[]],
        );
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keypress(KeyCode::B, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::B, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(
            &keyboard,
            &[&[KeyCode::X], &[KeyCode::X, KeyCode::B], &[KeyCode::X], &[]],
        );
        keyboard.output.clear();
        keyboard.output.state().disable_handler(layer_id);
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::A], &[]]);
        keyboard.output.clear();
        keyboard.output.state().enable_handler(layer_id);
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::X], &[]]);

        //TODO: what happens when you disable the layer in the middle?
        // I suspect that we will keep repeating one of the keycodes.
        // what would be the sensible thing to happen? How can we achive this?
        // possibly by clearing the keyboard events whenever a layer toggle happens?
    }
    #[test]
    fn test_layer_rewrite_shifted() {
        let l = Layer::new(vec![(
            KeyCode::A,
            LayerAction::RewriteToShifted(KeyCode::M.into(), KeyCode::Z.into()))],
            AutoOff::No,
        );
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let layer_id = keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        keyboard.output.state().enable_handler(layer_id);
        assert!(!keyboard.output.state().modifier(Shift));
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::M], &[]]);
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::LShift, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(
            &keyboard,
            &[&[KeyCode::LShift], &[KeyCode::LShift, KeyCode::Z]],
        );
        assert!(keyboard.output.state().modifier(Shift));
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift]]);
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::LShift, 0);
        keyboard.handle_keys().unwrap();
        dbg!(keyboard.output.state());
        assert!(!(keyboard.output.state().modifier(Shift)));
        check_output(&keyboard, &[&[]]);
    }
    #[test]
    fn test_layer_double_rewrite() {
        use crate::handlers::LayerAction::RewriteTo;
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let l = Layer::new(vec![
            (KeyCode::A, RewriteTo(KeyCode::B.to_u32())),
            (KeyCode::B, RewriteTo(KeyCode::C.to_u32())),
        ],
        AutoOff::No,
        );
        let layer_id = keyboard.add_handler(Box::new(l));
        assert!(!keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.output.state().enable_handler(layer_id);
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::B]]);
    }
    #[test]
    fn test_layer_disable_in_the_middle() {
        use crate::handlers::LayerAction::RewriteTo;
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let l = Layer::new(vec![(KeyCode::A, RewriteTo(KeyCode::B.to_u32()))],
        AutoOff::No);
        let layer_id = keyboard.add_handler(Box::new(l));
        assert!(!keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.output.state().enable_handler(layer_id);
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::B]]);
        keyboard.output.clear();

        keyboard.output.state().disable_handler(layer_id);
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();
    }
    #[test]
    fn test_rewrite_shifted() {
        use crate::handlers::LayerAction::RewriteToShifted;
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let l = Layer::new(vec![(KeyCode::A, RewriteToShifted(0xC6, 0xF6))], AutoOff::No);
        let layer_id = keyboard.add_handler(Box::new(l));
        assert!(!keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.output.state().enable_handler(layer_id);
        keyboard.output.state().unicode_mode = UnicodeSendMode::Debug;
        keyboard.add_handler(Box::new(crate::test_helpers::Debugger::new("A")));
        keyboard.add_handler(Box::new(UnicodeKeyboard::new()));
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::C], &[KeyCode::Kp6], &[]]);
        keyboard.output.clear();

        keyboard.output.state().set_modifier(Shift, true);
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.add_keyrelease(KeyCode::A, 0);

        keyboard.handle_keys().unwrap();
        check_output(
            &keyboard,
            &[&[KeyCode::F], &[KeyCode::Kp6], &[KeyCode::LShift]],
        );
        keyboard.output.clear();
    }

    #[test]
    fn test_layer_rewrite_unicode() {
        let l = Layer::new(vec![(KeyCode::A, LayerAction::RewriteTo(0xDF))], AutoOff::No);
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(crate::test_helpers::Debugger::new("start")));
        keyboard.output.state().unicode_mode = UnicodeSendMode::Debug;
        let layer_id = keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(crate::test_helpers::Debugger::new("bu")));
        keyboard.add_handler(Box::new(UnicodeKeyboard::new()));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        keyboard.output.state().enable_handler(layer_id);
        keyboard.add_keypress(KeyCode::J, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::J]]);
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::J]]);
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::D], &[KeyCode::F], &[KeyCode::J]]);
    }

    #[test]
    fn test_rewrite_shifted_string() {
        use crate::handlers::LayerAction::SendStringShifted;
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let l = Layer::new(vec![(KeyCode::A, SendStringShifted("a", "A"))], AutoOff::No);
        let layer_id = keyboard.add_handler(Box::new(l));
        assert!(!keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.output.state().enable_handler(layer_id);
        keyboard.output.state().unicode_mode = UnicodeSendMode::Debug;
        keyboard.add_handler(Box::new(crate::test_helpers::Debugger::new("A")));
        keyboard.add_handler(Box::new(UnicodeKeyboard::new()));
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::Kp6], &[KeyCode::Kp1], &[]]);
        keyboard.output.clear();

        keyboard.output.state().set_modifier(Shift, true);
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.add_keyrelease(KeyCode::A, 0);

        keyboard.handle_keys().unwrap();
        check_output(
            &keyboard,
            &[&[KeyCode::Kp4], &[KeyCode::Kp1], &[KeyCode::LShift] ], //the shift get's send from the USBKeyboard instead of an empty report...
        );
        keyboard.output.clear();
    }

   #[test]
    fn test_layer_auto_off_after_all() {
        use crate::test_helpers::Checks;
        use crate::key_codes::KeyCode::*;
        let l = Layer::new(vec![(
            A,
            LayerAction::RewriteTo(X.into()),)],
            AutoOff::AfterAll,
        );
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let layer_id = keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        keyboard.output.state().enable_handler(layer_id);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.pc(B, &[&[B]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.rc(B, &[&[]]);
        assert!(!keyboard.output.state().is_handler_enabled(layer_id));

        keyboard.output.state().enable_handler(layer_id);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.pc(A, &[&[X]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.rc(A, &[&[]]);
        assert!(!keyboard.output.state().is_handler_enabled(layer_id));

        keyboard.output.state().enable_handler(layer_id);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.pc(LShift, &[&[LShift]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.rc(LShift, &[&[]]);
        assert!(!keyboard.output.state().is_handler_enabled(layer_id));
    }

    #[test]
    fn test_layer_auto_off_after_match() {
        use crate::test_helpers::Checks;
        use crate::key_codes::KeyCode::*;
        let l = Layer::new(vec![(
            A,
            LayerAction::RewriteTo(X.into()),)],
            AutoOff::AfterMatch,
        );
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let layer_id = keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        keyboard.output.state().enable_handler(layer_id);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.pc(B, &[&[B]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.rc(B, &[&[]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));

        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.pc(A, &[&[X]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.rc(A, &[&[]]);
        assert!(!keyboard.output.state().is_handler_enabled(layer_id));

        keyboard.output.state().enable_handler(layer_id);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.pc(LShift, &[&[LShift]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.rc(LShift, &[&[]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));

        keyboard.pc(LShift, &[&[LShift]]);
        keyboard.pc(A, &[&[LShift, X]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.rc(A, &[&[LShift]]);
        assert!(!keyboard.output.state().is_handler_enabled(layer_id));

        keyboard.rc(LShift, &[&[]]);
        assert!(!keyboard.output.state().is_handler_enabled(layer_id));
    }

    #[test]
    fn test_layer_auto_off_after_non_modifier() {
        use crate::test_helpers::Checks;
        use crate::key_codes::KeyCode::*;
        use crate::key_codes::UserKey;
        use crate::premade::one_shot_handler;
        let l = Layer::new(vec![(
            A,
            LayerAction::RewriteTo(X.into()),)],
            AutoOff::AfterNonModifier,
        );
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let layer_id = keyboard.add_handler(Box::new(l));

        let l2 = Layer::new(vec![(
            C,
            LayerAction::RewriteTo(Y.into()),)],
            AutoOff::No,
        );
        let layer_id2 = keyboard.add_handler(Box::new(l2));
        let oneshot = one_shot_handler(UserKey::UK0, layer_id2, 0, 0);

        keyboard.add_handler(oneshot);
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        keyboard.output.state().enable_handler(layer_id);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.pc(B, &[&[B]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.rc(B, &[&[]]);
        assert!(!keyboard.output.state().is_handler_enabled(layer_id));

        keyboard.output.state().enable_handler(layer_id);
        keyboard.pc(A, &[&[X]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.rc(A, &[&[]]);
        assert!(!keyboard.output.state().is_handler_enabled(layer_id));

        keyboard.output.state().enable_handler(layer_id);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.pc(LShift, &[&[LShift]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.rc(LShift, &[&[]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));

        keyboard.pc(LShift, &[&[LShift]]);
        keyboard.pc(A, &[&[LShift, X]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.rc(A, &[&[LShift]]);
        assert!(!keyboard.output.state().is_handler_enabled(layer_id));

        keyboard.rc(LShift, &[&[]]);
        assert!(!keyboard.output.state().is_handler_enabled(layer_id));

        keyboard.output.state().enable_handler(layer_id);
        keyboard.pc(UserKey::UK0, &[&[]]);
        keyboard.rc(UserKey::UK0, &[&[]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        assert!(keyboard.output.state().is_handler_enabled(layer_id2));
        keyboard.pc(C, &[&[Y]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        assert!(keyboard.output.state().is_handler_enabled(layer_id2));
        keyboard.rc(C, &[&[]]);
        assert!(!keyboard.output.state().is_handler_enabled(layer_id));
        assert!(!keyboard.output.state().is_handler_enabled(layer_id2));


        keyboard.output.state().enable_handler(layer_id);
        keyboard.pc(UserKey::UK0, &[&[]]);
        keyboard.rc(UserKey::UK0, &[&[]]);
        keyboard.pc(LShift, &[&[LShift]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        assert!(keyboard.output.state().is_handler_enabled(layer_id2));
        keyboard.pc(D, &[&[D, LShift]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        //disabled by the one shot
        assert!(!keyboard.output.state().is_handler_enabled(layer_id2));
        keyboard.rc(D, &[&[LShift]]);
        assert!(!keyboard.output.state().is_handler_enabled(layer_id));
        assert!(!keyboard.output.state().is_handler_enabled(layer_id2));
        keyboard.rc(LShift, &[&[]]);

        keyboard.output.state().enable_handler(layer_id);
        keyboard.pc(UserKey::UK0, &[&[]]);
        keyboard.rc(UserKey::UK0, &[&[]]);
        keyboard.pc(LShift, &[&[LShift]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        assert!(keyboard.output.state().is_handler_enabled(layer_id2));
        keyboard.pc(A, &[&[X, LShift]]);
        assert!(keyboard.output.state().is_handler_enabled(layer_id));
        assert!(!keyboard.output.state().is_handler_enabled(layer_id2));
        keyboard.rc(A, &[&[LShift]]);
        assert!(!keyboard.output.state().is_handler_enabled(layer_id));
        assert!(!keyboard.output.state().is_handler_enabled(layer_id2));




    }



}
