use no_std_compat::prelude::v1::*;
use crate::handlers::ProcessKeys;
use crate::key_codes::AcceptsKeycode;
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use crate::USBKeyOut;
#[derive(PartialEq)]
enum MatchResult<'a> {
    Match(&'a str),
    WontMatch,
    NeedsMoreInput,
}
pub struct Leader<'a> {
    trigger: u32,
    mappings: Vec<(Vec<u32>, &'a str)>,
    failure: &'a str,
    prefix: Vec<u32>, //todo: refactor to not need this but use repeated iterators?
    active: bool,
}
impl<'a> Leader<'a> {
    pub fn new<T: AcceptsKeycode>(
        trigger: impl AcceptsKeycode,
        mappings: Vec<(Vec<T>, &'a str)>,
        failure: &'a str,
    ) -> Leader<'a> {
        //Todo: Figure out how to check for mappings that are prefixes of other mappings
        //(and therefore impossible) at compile time
        Leader {
            trigger: trigger.to_u32(),
            mappings: mappings
                .into_iter()
                .map(|(a, b)| (a.into_iter().map(|x| x.to_u32()).collect(), b))
                .collect(),
            failure,
            prefix: Vec::new(),
            active: false,
        }
    }
    fn match_prefix(&self) -> MatchResult {
        let mut result = MatchResult::WontMatch;
        for (seq, out) in self.mappings.iter() {
            if seq.len() < self.prefix.len() {
                continue;
            }
            if self.prefix.iter().zip(seq.iter()).all(|(a, b)| a == b) {
                if seq.len() == self.prefix.len() {
                    return MatchResult::Match(out);
                } else {
                    result = MatchResult::NeedsMoreInput;
                }
            }
        }
        result
    }
}
impl<T: USBKeyOut> ProcessKeys<T> for Leader<'_> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
        for (event, status) in iter_unhandled_mut(events) {
            match event {
                Event::KeyRelease(kc) => {
                    if self.active {
                        self.prefix.push(kc.keycode);
                        match self.match_prefix() {
                            MatchResult::Match(s) => {
                                output.send_string(s);
                                self.active = false;
                                self.prefix.clear()
                            }
                            MatchResult::WontMatch => {
                                output.send_string(self.failure);
                                self.active = false;
                                self.prefix.clear()
                            }
                            MatchResult::NeedsMoreInput => {}
                        }
                        *status = EventStatus::Handled;
                    } else if kc.keycode == self.trigger {
                        if !self.active {
                            self.active = true;
                        }
                        *status = EventStatus::Handled;
                    }
                }
                Event::KeyPress(kc) => {
                    if kc.keycode == self.trigger {
                        *status = EventStatus::Handled;
                    } else if self.active {
                        // while active, we eat all KeyPresses and only parse KeyRelease
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
        leader::MatchResult, 
        Leader,
        USBKeyboard,
    };
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
    fn test_leader() {
        use crate::key_codes::KeyCode::*;
        use core::convert::TryInto;
        let mut l = Leader::new(
            KeyCode::X,
            vec![
                (vec![A, B, C], "A"),
                (vec![A, B, D], "B"),
                //Todo: check that none is a prefix of another!
                //(vec![A], "C"),
            ],
            "E",
        );
        assert!(l.match_prefix() == MatchResult::NeedsMoreInput);
        l.prefix.push(A.into());
        assert!(l.match_prefix() == MatchResult::NeedsMoreInput);
        l.prefix.push(B.into());
        assert!(l.match_prefix() == MatchResult::NeedsMoreInput);
        l.prefix.push(C.into());
        assert!(match l.match_prefix() {
            MatchResult::Match(m) => {
                assert!(m == "A");
                true
            }
            _ => false,
        });
        l.prefix.clear();
        assert!(l.match_prefix() == MatchResult::NeedsMoreInput);
        l.prefix.push(C.into());
        assert!(l.match_prefix() == MatchResult::WontMatch);
        l.prefix.clear();
        let keyb = USBKeyboard::new();
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(keyb));
        keyboard.output.state().unicode_mode = UnicodeSendMode::Debug;
        //activate
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::B, 0);
        keyboard.add_keyrelease(KeyCode::B, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::C, 0);
        keyboard.add_keyrelease(KeyCode::C, 0);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[65u8.try_into().unwrap()], &[]]);
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::F, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::F, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[F], &[]]);
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();
        //test error case
        keyboard.add_keypress(KeyCode::C, 0);
        keyboard.add_keyrelease(KeyCode::C, 0);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[69u8.try_into().unwrap()], &[]]);
    }
}
