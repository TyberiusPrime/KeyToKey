use crate::handlers::ProcessKeys;
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use crate::USBKeyOut;

use no_std_compat::prelude::v1::*;

/// A layer that *only* supports replacing key codes
/// with other key codes.
///
/// The advantage of this is that you can/must use it with a const
/// array (slice), which greatly saves on ram compared to Layer
/// (e.g. premade::dvorak)
///
pub struct RewriteLayer {
    rewrites: &'static [(u32, u32)],
}

impl RewriteLayer {
    pub fn new(rewrites: &'static [(u32, u32)]) -> RewriteLayer {
        RewriteLayer { rewrites }
    }
}

impl<T: USBKeyOut> ProcessKeys<T> for RewriteLayer {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, _output: &mut T) {
        for (event, _status) in iter_unhandled_mut(events) {
            //events.iter_mut() {
            match event {
                Event::KeyRelease(kc) => {
                    for (from, to) in self.rewrites.iter() {
                        if *from == kc.keycode {
                            if (kc.flag & 2) == 0 {
                                kc.keycode = *to;
                                kc.flag |= 2;
                            }
                            break; //only one rewrite per layer
                        }
                    }
                }
                Event::KeyPress(kc) => {
                    for (from, to) in self.rewrites.iter() {
                        if *from == kc.keycode {
                            if (kc.flag & 2) == 0 {
                                kc.keycode = *to;
                                kc.flag |= 2;
                            }
                            break; //only one rewrite per layer
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
    use crate::handlers::{RewriteLayer, USBKeyboard, UnicodeKeyboard};
    use crate::key_codes::KeyCode;
    use crate::test_helpers::{check_output, KeyOutCatcher};
    use crate::{Keyboard, USBKeyOut, UnicodeSendMode};
    #[allow(unused_imports)]
    use no_std_compat::prelude::v1::*;

    #[test]
    fn test_layer_rewrite() {
        const MAP: &[(u32, u32)] = &[(KeyCode::A.to_u32(), KeyCode::X.to_u32())];
        let l = RewriteLayer::new(&MAP);
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
    }

    #[test]
    fn test_layer_double_rewrite() {
        const MAP: &[(u32, u32)] = &[
            (KeyCode::A.to_u32(), KeyCode::B.to_u32()),
            (KeyCode::B.to_u32(), KeyCode::C.to_u32()),
        ];
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let l = RewriteLayer::new(&MAP);
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
        const MAP: &[(u32, u32)] = &[(KeyCode::A.to_u32(), KeyCode::B.to_u32())];
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let l = RewriteLayer::new(&MAP);
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
    fn test_layer_rewrite_unicode() {
        const MAP: &[(u32, u32)] = &[(KeyCode::A.to_u32(), 0xDF)];
        let l = RewriteLayer::new(&MAP);
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.output.state().unicode_mode = UnicodeSendMode::Debug;
        let layer_id = keyboard.add_handler(Box::new(l));
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
