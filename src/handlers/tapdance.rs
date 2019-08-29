use crate::handlers::{ProcessKeys, HandlerResult};
use crate::key_codes::AcceptsKeycode;
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use crate::USBKeyOut;
use no_std_compat::prelude::v1::*;

pub enum TapDanceEnd {
    Timeout,
    OtherKey
}

/// call backs for completed tap dances
pub trait TapDanceAction {
    fn on_tapdance( &mut self, trigger: u32, output: &mut impl USBKeyOut, tap_count: u8, tap_end: TapDanceEnd);
}


pub struct TapDance<M>{
    trigger: u32,
    tap_count: u8,
    action: M,
    //todo: add on_each_tap...
    timeout_ms: u16,
}

impl <M: TapDanceAction> TapDance<M> {
    pub fn new(trigger: impl AcceptsKeycode, action: M, timeout_ms: u16) -> TapDance<M> {
        TapDance {
            trigger: trigger.to_u32(),
            tap_count: 0,
            action,
            timeout_ms: timeout_ms,
        }
    }
}
impl<T: USBKeyOut, M: TapDanceAction> ProcessKeys<T> for TapDance<M> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) ->HandlerResult {
        for (event, status) in iter_unhandled_mut(events) {
            match event {
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.trigger {
                        *status = EventStatus::Handled;
                    }
                }
                Event::KeyPress(kc) => {
                    if kc.keycode != self.trigger {
                        if self.tap_count > 0 {
                            self.action.on_tapdance(self.trigger, output, self.tap_count, TapDanceEnd::OtherKey);
                            self.tap_count = 0;
                        }
                    } else {
                        self.tap_count += 1;
                        *status = EventStatus::Handled;
                    }
                }
                Event::TimeOut(ms_since_last) => {
                    if self.tap_count > 0 && *ms_since_last >= self.timeout_ms {
                            self.action.on_tapdance(self.trigger, output, self.tap_count, TapDanceEnd::Timeout);
                        self.tap_count = 0;
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
    use crate::handlers::{TapDance, USBKeyboard, TapDanceAction, TapDanceEnd};
    #[allow(unused_imports)]
    use crate::key_codes::KeyCode;
    #[allow(unused_imports)]
    use crate::test_helpers::{check_output, KeyOutCatcher, Checks};
    #[allow(unused_imports)]
    use crate::{
        Event, EventStatus, Keyboard, KeyboardState, ProcessKeys, USBKeyOut, UnicodeSendMode,
    };
    #[allow(unused_imports)]
    use no_std_compat::prelude::v1::*;
    use alloc::sync::Arc;
    use spin::RwLock;

    #[derive(Debug)]
    pub struct TapDanceLogger {
        pub other_key_taps: u16,
        pub timeout_taps: u16,
    }
    impl TapDanceLogger {
        fn new() -> TapDanceLogger {
            TapDanceLogger{other_key_taps: 0, timeout_taps: 0}
        }
    }
    impl TapDanceAction for Arc<RwLock<TapDanceLogger>> {
        fn on_tapdance( &mut self, _trigger: u32, output: &mut impl USBKeyOut, tap_count: u8, tap_end: TapDanceEnd){
            match tap_end {
                TapDanceEnd::OtherKey => self.write().other_key_taps += tap_count as u16,
                TapDanceEnd::Timeout => self.write().timeout_taps += tap_count as u16,
            }
            output.send_keys(&[KeyCode::A]);
        }
    }

    #[test]
    fn test_tapdance() {
        let record = Arc::new(RwLock::new(TapDanceLogger::new()));
        let l = TapDance::new( KeyCode::X, record.clone(), 250);
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        //simplest case, one press/release then another key
        keyboard.pc(KeyCode::X, &[&[]]);
        assert!(record.read().other_key_taps == 0);
        assert!(record.read().timeout_taps == 0);

        keyboard.rc(KeyCode::X, &[&[]]);
        assert!(record.read().other_key_taps == 0);
        assert!(record.read().timeout_taps == 0);

        keyboard.pc(KeyCode::Z, &[&[KeyCode::A], &[KeyCode::Z]]);
        assert!(record.read().other_key_taps == 1);
        assert!(record.read().timeout_taps == 0);

        keyboard.rc(KeyCode::Z, &[&[]]);
        assert!(record.read().other_key_taps == 1);
        assert!(record.read().timeout_taps == 0);

        //two taps, then another key
        keyboard.pc(KeyCode::X, &[&[]]);
        keyboard.rc(KeyCode::X, &[&[]]);
        keyboard.pc(KeyCode::X, &[&[]]);
        keyboard.rc(KeyCode::X, &[&[]]);
        assert!(record.read().other_key_taps == 1);
       assert!(record.read().timeout_taps == 0);

        keyboard.pc(KeyCode::Z, &[&[KeyCode::A], &[KeyCode::Z]]);
        assert!(record.read().other_key_taps == 3);
        assert!(record.read().timeout_taps == 0);

        keyboard.rc(KeyCode::Z, &[&[]]);
        assert!(record.read().other_key_taps == 3);
        assert!(record.read().timeout_taps == 0);



        //three taps, then a time out
        keyboard.pc(KeyCode::X, &[&[]]);
        keyboard.rc(KeyCode::X, &[&[]]);
        keyboard.pc(KeyCode::X, &[&[]]);
        keyboard.rc(KeyCode::X, &[&[]]);
        keyboard.pc(KeyCode::X, &[&[]]);
        keyboard.rc(KeyCode::X, &[&[]]);
        assert!(record.read().other_key_taps == 3);
        assert!(record.read().timeout_taps == 0);
        //not a timeout...
        keyboard.tc(249, &[&[]]); //remember, repeaeted empty ones are (supossed to be) ommited by the downstream USB handling
        keyboard.tc(250, &[&[KeyCode::A], &[]]);
        assert!(record.read().other_key_taps == 3);
        assert!(record.read().timeout_taps == 3);
    }
}
