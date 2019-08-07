use no_std_compat::prelude::v1::*;
use crate::handlers::ProcessKeys;
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use crate::USBKeyOut;
/// This processor sends unicode 'characters'
/// just map your keys to unicode 'code points'
/// sending happens on keyrelease - no key repeat
///
/// the private ranges of unicode are not send,
/// but some of them are intpreted as USB-Keycodes
/// by UsbKeyboard.
/// Use UserKey::* for totally custom keys
pub struct UnicodeKeyboard {}
impl UnicodeKeyboard {
    fn is_unicode_keycode(keycode: u32) -> bool {
        match keycode {
            0x100000..=0x1000FF => false, //these are the usb codes
            0xF0000..=0xFFFFD => false,   //unicode private character range A
            0x1000FF..=0x10FFFD => false, //unicode private character range b (minus those we use for codes < 256)
            _ => true,
        }
    }
    fn keycode_to_unicode(keycode: u32) -> u32 {
        if keycode < 0x100000 {
            keycode
        } else {
            keycode - 0x100000
        }
    }
}
impl<T: USBKeyOut> ProcessKeys<T> for UnicodeKeyboard {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
        for (event, status) in iter_unhandled_mut(events) {
            match event {
                Event::KeyPress(kc) => {
                    if UnicodeKeyboard::is_unicode_keycode(kc.keycode) {
                        *status = EventStatus::Handled;
                    }
                }
                Event::KeyRelease(kc) => {
                    if UnicodeKeyboard::is_unicode_keycode(kc.keycode) {
                        let c = no_std_compat::char::from_u32(UnicodeKeyboard::keycode_to_unicode(
                            kc.keycode,
                        ));
                        if let Some(c) = c {
                            output.send_unicode(c);
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
    use crate::handlers::{USBKeyboard, UnicodeKeyboard};
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
    fn test_unicode_keyboard_linux() {
        use crate::key_codes::KeyCode::*;
        let ub = UnicodeKeyboard {};
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(ub));
        keyboard.output.state().unicode_mode = UnicodeSendMode::Linux;
        //no output on press
        keyboard.add_keypress(0x00E4u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.reports.len() == 0);
        assert!(keyboard.events.is_empty()); // we eat the keypress though
        keyboard.add_keyrelease(0x00E4, 0);
        keyboard.handle_keys().unwrap();
        check_output(
            &keyboard,
            &[
                &[U, LShift, LCtrl],
                &[E, LShift, LCtrl],
                &[Kb4, LShift, LCtrl],
                &[],
            ],
        );
        assert!(keyboard.events.is_empty()); // we eat the keypress though
    }
    #[test]
    fn test_unicode_keyboard_wincompose() {
        use crate::key_codes::KeyCode::*;
        let ub = UnicodeKeyboard {};
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(ub));
        keyboard.output.state().unicode_mode = UnicodeSendMode::WinCompose;
        //no output on press
        keyboard.add_keypress(0x03B4u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.reports.len() == 0);
        assert!(keyboard.events.is_empty()); // we eat the keypress though
        keyboard.add_keyrelease(0x03B4, 0);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(
            &keyboard,
            &[&[RAlt], &[U], &[Kb3], &[B], &[Kb4], &[Enter], &[]],
        );
        assert!(keyboard.events.is_empty()); // we eat the keypress though
    }
    #[test]
    fn test_unicode_while_depressed() {
        use crate::key_codes::KeyCode::*;
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(UnicodeKeyboard {}));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        keyboard.output.state().unicode_mode = UnicodeSendMode::WinCompose;
        keyboard.add_keypress(A, 0);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[A]]);
        keyboard.output.clear();
        keyboard.add_keypress(0x3B4u32, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[A]]);
        keyboard.add_keyrelease(0x3B4, 0);
        keyboard.output.clear();
        keyboard.handle_keys().unwrap();
        check_output(
            &keyboard,
            &[&[RAlt], &[U], &[Kb3], &[B], &[Kb4], &[Enter], &[], &[A]],
        );
        keyboard.add_keyrelease(A, 0);
        keyboard.output.clear();
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        assert!(keyboard.events.is_empty());
    }
}
