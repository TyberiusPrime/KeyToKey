use crate::handlers::{Action, ProcessKeys};
use crate::key_codes::{KeyCode, KeyCodeInfo};
use crate::{iter_unhandled_mut, Event, EventStatus, USBKeyOut};
use no_std_compat::prelude::v1::*;

/// A sequence is a series of keystrokes (press and release)
/// that upon finish (ie. the release of the last key)
/// sends first a (configurable) number of backspaces (to undo the input)
/// and then an action.
/// 
/// sequence keys - even if matching are not handled by Sequence,
/// except if they're from the private range, in which 
/// case the Sequence will consume the Events.
/// 
/// It is suggested to prefix your sequences with a unicode symbol,
/// so you can observe the feedback.
/// 
/// Note that for a final KeyCode::*, you will need to send a backspace,
/// but for a final unicode (or private) one you don't.
pub struct Sequence<'a, M> {
    sequence: &'a [u32],
    callback: M,
    backspaces: u8,
    pos: u8,
}

impl<'a, M: Action> Sequence<'a, M> {
    pub fn new(sequence: &'a [u32], callback: M, backspaces: u8) -> Sequence<'a, M> {
        if sequence.len() > 254 {
            panic!("Sequence too long, max 254 key codes");
        }
        Sequence {
            sequence,
            callback,
            backspaces,
            pos: 0,
        }
    }
}

impl<T: USBKeyOut, M: Action> ProcessKeys<T> for Sequence<'_, M> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) {
        let mut codes_to_delete: Vec<u32> = Vec::new();
        for (event, status) in iter_unhandled_mut(events).rev() {
            match event {
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.sequence[self.pos as usize] {
                        if kc.keycode.is_private_keycode() {
                            *status = EventStatus::Handled;
                        }
                        self.pos += 1;
                        if self.pos == self.sequence.len() as u8 {
                            self.pos = 0;
                            for _ in 0..self.backspaces {
                                output.send_keys(&[KeyCode::BSpace]);
                                output.send_empty();
                            }
                            self.callback.on_trigger(output);
                            *status = EventStatus::Handled;
                            if !codes_to_delete.contains(&kc.original_keycode) {
                                codes_to_delete.push(kc.original_keycode);
                            }
                        }

                    //todo: remove matching key pres
                    } else {
                        self.pos = 0;
                    }
                }
                Event::KeyPress(kc) => {
                    if codes_to_delete.contains(&kc.original_keycode) {
                        *status = EventStatus::Handled;
                    }
                    if kc.keycode == self.sequence[self.pos as usize] && kc.keycode.is_private_keycode() {
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
    use crate::handlers::{Sequence, USBKeyboard, UnicodeKeyboard};
    #[allow(unused_imports)]
    use crate::key_codes::{KeyCode, UserKey};
    #[allow(unused_imports)]
    use crate::test_helpers::{check_output, Checks, KeyOutCatcher};
    use crate::{
        Event, EventStatus, Keyboard, KeyboardState, ProcessKeys, USBKeyOut, UnicodeSendMode,
    };
    #[allow(unused_imports)]
    use no_std_compat::prelude::v1::*;
    #[test]
    fn test_sequence() {
        use crate::key_codes::KeyCode::*;

        let map = &[A.to_u32(), B.to_u32(), C.to_u32()];
        let l = Sequence::new(map, X, 3);
        let mut k = Keyboard::new(KeyOutCatcher::new());
        k.output.state().unicode_mode = UnicodeSendMode::Debug;
        k.add_handler(Box::new(l));
        k.add_handler(Box::new(USBKeyboard::new()));

        k.pc(A, &[&[A]]);
        k.rc(A, &[&[]]);

        k.pc(B, &[&[B]]);
        k.rc(B, &[&[]]);

        k.pc(C, &[&[C]]);
        k.rc(C, &[&[BSpace], &[], &[BSpace], &[], &[BSpace], &[], &[X]]);

        k.pc(A, &[&[A]]);
        k.rc(A, &[&[]]);

        k.pc(B, &[&[B]]);
        k.rc(B, &[&[]]);

        k.pc(D, &[&[D]]);
        k.rc(D, &[&[]]);

        k.pc(B, &[&[B]]);
        k.rc(B, &[&[]]);

        k.pc(C, &[&[C]]);
        k.rc(C, &[&[]]);

        k.pc(A, &[&[A]]);
        k.rc(A, &[&[]]);

        k.pc(B, &[&[B]]);
        k.rc(B, &[&[]]);

        k.pc(C, &[&[C]]);
        k.rc(C, &[&[BSpace], &[], &[BSpace], &[], &[BSpace], &[], &[X]]);


    }

    #[test]
    fn test_sequence_unicode_trigger() {
        use crate::key_codes::KeyCode::*;

        let map = &[0xDF, B.to_u32(), C.to_u32()];
        let l = Sequence::new(map, X, 3);
        let mut k = Keyboard::new(KeyOutCatcher::new());
        k.output.state().unicode_mode = UnicodeSendMode::Debug;
        k.add_handler(Box::new(l));
        k.add_handler(Box::new(UnicodeKeyboard::new()));
        k.add_handler(Box::new(USBKeyboard::new()));

        k.pc(0xDF, &[&[]]);
        k.rc(0xDF, &[&[D], &[F], &[]]);

        k.pc(B, &[&[B]]);
        k.rc(B, &[&[]]);

        k.pc(C, &[&[C]]);
        k.rc(C, &[&[BSpace], &[], &[BSpace], &[], &[BSpace], &[], &[X]]);

    }
    #[test]
    fn test_sequence_private_trigger() {
        use crate::key_codes::KeyCode::*;

        let map = &[UserKey::UK1.to_u32(), B.to_u32(), C.to_u32()];
        let l = Sequence::new(map, X, 3);
        let mut k = Keyboard::new(KeyOutCatcher::new());
        k.output.state().unicode_mode = UnicodeSendMode::Debug;
        k.add_handler(Box::new(l));
        k.add_handler(Box::new(UnicodeKeyboard::new()));
        k.add_handler(Box::new(USBKeyboard::new()));

        k.pc(UserKey::UK1, &[&[]]);
        k.rc(UserKey::UK1, &[&[]]);

        k.pc(B, &[&[B]]);
        k.rc(B, &[&[]]);

        k.pc(C, &[&[C]]);
        k.rc(C, &[&[BSpace], &[], &[BSpace], &[], &[BSpace], &[], &[X]]);
    }
    #[test]
    fn test_sequence_mixed_trigger() {
        use crate::key_codes::KeyCode::*;
        let map = &[A.to_u32(), UserKey::UK1.to_u32(), 0x1234];
        let l = Sequence::new(map, X, 1);
        let mut k = Keyboard::new(KeyOutCatcher::new());
        k.output.state().unicode_mode = UnicodeSendMode::Debug;
        k.add_handler(Box::new(l));
        k.add_handler(Box::new(UnicodeKeyboard::new()));
        k.add_handler(Box::new(USBKeyboard::new()));

        k.pc(A, &[&[A]]);
        k.rc(A, &[&[]]);

        k.pc(UserKey::UK1, &[&[]]);
        k.rc(UserKey::UK1, &[&[]]);

        k.pc(0x1234, &[&[]]);
        k.rc(0x1234, &[
            &[BSpace], &[], 
            &[X]]);
    }



}
