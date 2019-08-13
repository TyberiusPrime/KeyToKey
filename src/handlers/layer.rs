use crate::handlers::ProcessKeys;
use crate::key_codes::AcceptsKeycode;
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use crate::Modifier::*;
use crate::USBKeyOut;

use no_std_compat::prelude::v1::*;
pub enum LayerAction<'a> {
    RewriteTo(u32),
    RewriteToShifted(u32, u32),
    //todo: rewrite shift
    SendString(&'a str),
    SendStringShifted(&'a str, &'a str),
    //    Callback(fn(&mut T) -> (), fn(&mut T) -> ()),
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
///
///
pub struct Layer<'a> {
    rewrites: Vec<(u32, LayerAction<'a>)>,
}
impl Layer<'_> {
    pub fn new<F: AcceptsKeycode>(rewrites: Vec<(F, LayerAction)>) -> Layer<'_> {
        Layer {
            rewrites: rewrites
                .into_iter()
                .map(|(trigger, action)| (trigger.to_u32(), action))
                .collect(),
        }
    }
}
impl<T: USBKeyOut> ProcessKeys<T> for Layer<'_> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) {
        for (event, status) in iter_unhandled_mut(events) {
            //events.iter_mut() {
            match event {
                Event::KeyRelease(kc) => {
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
                                LayerAction::SendString(s) => {
                                    output.send_string(s);
                                    *status = EventStatus::Handled;
                                    break; //only one rewrite per layer
                                }
                                LayerAction::SendStringShifted(s1, s2) => {
                                    if output.state().modifier(Shift) {
                                        output.send_string(s2);
                                    } else {
                                        output.send_string(s1);
                                    }
                                    *status = EventStatus::Handled;
                                    break; //only one rewrite per layer
                                }
                            }
                        }
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
    }
    fn default_enabled(&self) -> bool {
        false
    }
}
#[cfg(test)]
//#[macro_use]
//extern crate std;
mod tests {
    use crate::handlers::{Layer, LayerAction, USBKeyboard, UnicodeKeyboard};
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
            LayerAction::RewriteTo(KeyCode::X.into()),
        )]);
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
            LayerAction::RewriteToShifted(KeyCode::M.into(), KeyCode::Z.into()),
        )]);
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
        ]);
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
        let l = Layer::new(vec![(KeyCode::A, RewriteTo(KeyCode::B.to_u32()))]);
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
        let l = Layer::new(vec![(KeyCode::A, RewriteToShifted(0xC6, 0xF6))]);
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
        let l = Layer::new(vec![(KeyCode::A, LayerAction::RewriteTo(0xDF))]);
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
}
