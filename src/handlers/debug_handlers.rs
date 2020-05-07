use crate::handlers::{ProcessKeys, HandlerResult};
use crate::key_codes::KeyCode;
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use crate::USBKeyOut;
///handlers that probably are only useful while building a keyboard
///
///
///
use no_std_compat::prelude::v1::*;
fn nibble_to_keycode(nibble: u8) -> KeyCode {
    match nibble {
        0 => KeyCode::Kb0,
        1 => KeyCode::Kb1,
        2 => KeyCode::Kb2,
        3 => KeyCode::Kb3,
        4 => KeyCode::Kb4,
        5 => KeyCode::Kb5,
        6 => KeyCode::Kb6,
        7 => KeyCode::Kb7,
        8 => KeyCode::Kb8,
        9 => KeyCode::Kb9,
        0xA => KeyCode::A,
        0xB => KeyCode::B,
        0xC => KeyCode::C,
        0xD => KeyCode::D,
        0xE => KeyCode::E,
        0xF => KeyCode::F,
        _ => {
            panic!("nibble larger than 0xF");
        }
    }
}
fn transform_u32_to_keycodes(x: u32) -> [KeyCode; 8] {
    [
        nibble_to_keycode(((x >> (32 - 4)) & 0xf) as u8),
        nibble_to_keycode(((x >> (32 - 8)) & 0xf) as u8),
        nibble_to_keycode(((x >> (32 - 12)) & 0xf) as u8),
        nibble_to_keycode(((x >> (32 - 16)) & 0xf) as u8),
        nibble_to_keycode(((x >> (32 - 20)) & 0xf) as u8),
        nibble_to_keycode(((x >> (32 - 24)) & 0xf) as u8),
        nibble_to_keycode(((x >> (32 - 28)) & 0xf) as u8),
        nibble_to_keycode((x & 0xf) as u8),
    ]
}
/// this handler helps you build a translation table for MatrixToStream
/// by outputing the keycode observed as
/// .into()<Enter>Hex-Keycode\tKeyCode::
/// so you can simply enter the keycode on a different
/// keyboard after pressing a key and later sort by
pub struct TranslationHelper {}
impl<T: USBKeyOut> ProcessKeys<T> for TranslationHelper {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) ->HandlerResult {
        for (e, status) in iter_unhandled_mut(events) {
            *status = EventStatus::Handled;
            match e {
                Event::KeyRelease(kc) => {
                    output.send_string(".into(),");
                    output.send_keys(&[KeyCode::Enter]);
                    output.send_empty();
                    let codes = transform_u32_to_keycodes(kc.keycode);
                    for c in &codes {
                        output.send_keys(&[*c]);
                        output.send_empty();
                    }
                    output.send_string("\tKeyCode::");
                    *status = EventStatus::Handled;
                }
                _ => {
                    *status = EventStatus::Handled;
                }
            };
        }
    HandlerResult::NoOp
    }
}
/// Debug a keystream at any point in the handling
/// by adding a DebugStream with a callback that knows
/// how to write something.
///
/// Omits Timeout Events, does not print empty keystreams
pub struct DebugStream<F> {
    pub write_callback: F,
}
impl<T: USBKeyOut, F: FnMut(String)> ProcessKeys<T> for DebugStream<F> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, _output: &mut T) ->HandlerResult {
        if !events.is_empty() {
            (self.write_callback)("[\n".to_string());
            for (e, status) in events.iter() {
                match e {
                    Event::KeyRelease(kc) => {
                        (self.write_callback)(format!(
                            "\t(Event::KeyRelease(Key::new({}, {}, {}, {})",
                            kc.keycode, kc.ms_since_last, kc.running_number, kc.flag,
                        ));
                    }
                    Event::KeyPress(kc) => {
                        (self.write_callback)(format!(
                            "\t(Event::KeyPress(Key::new({}, {}, {}, {})",
                            kc.keycode, kc.ms_since_last, kc.running_number, kc.flag,
                        ));
                    }
                    Event::TimeOut(_) => {}
                };
                match status {
                    EventStatus::Handled => {
                        (self.write_callback)("EventStatus::Handled),".to_string())
                    }
                    EventStatus::Unhandled => {
                        (self.write_callback)("EventStatus::Unhandled),".to_string())
                    }
                    EventStatus::Ignored => {
                        (self.write_callback)("EventStatus::Ignored),".to_string())
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
    use crate::handlers::debug_handlers::transform_u32_to_keycodes;
    use crate::key_codes::KeyCode;
    #[test]
    fn test_transform_u32_to_keycodes() {
        assert!(transform_u32_to_keycodes(0) == [KeyCode::Kb0; 8]);
        assert!(
            transform_u32_to_keycodes(1)
                == [
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb1,
                ]
        );
        assert!(
            transform_u32_to_keycodes(10)
                == [
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::A,
                ]
        );
        assert!(
            transform_u32_to_keycodes(16)
                == [
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb1,
                    KeyCode::Kb0,
                ]
        );
        assert!(
            transform_u32_to_keycodes(255)
                == [
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::F,
                    KeyCode::F,
                ]
        );
        dbg!(transform_u32_to_keycodes(255));
        assert!(
            transform_u32_to_keycodes(256 + 0xA2)
                == [
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb0,
                    KeyCode::Kb1,
                    KeyCode::A,
                    KeyCode::Kb2,
                ]
        );
    }
}
