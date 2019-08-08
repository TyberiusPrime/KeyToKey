/// premade handlers for various occacions
use crate::handlers::{Layer, OnOff, OneShot, PressReleaseMacro, SpaceCadet, Action};
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use crate::Modifier::*;
use crate::{AcceptsKeycode, HandlerID, KeyCode, ProcessKeys, USBKeyOut};
use no_std_compat::prelude::v1::*;
///toggle a handler on activate
/// do noting on deactivate
/// probably only usefull with PressReleaseMacro
/// used by toggle_handler()
pub struct ActionToggleHandler {
    id: HandlerID,
}
impl OnOff for ActionToggleHandler {
    fn on_activate(&mut self, output: &mut impl USBKeyOut) {
        output.state().toggle_handler(self.id);
    }
    fn on_deactivate(&mut self, _output: &mut impl USBKeyOut) {}
}

impl Action for ActionToggleHandler {
    fn on_trigger(&mut self, output: &mut impl USBKeyOut) {
        output.state().toggle_handler(self.id);
    }
}

/// Toggles a handler on and off when a key is pressed
pub fn toggle_handler(
    trigger: impl AcceptsKeycode,
    id: HandlerID,
) -> Box<PressReleaseMacro<ActionToggleHandler>> {
    Box::new(PressReleaseMacro::new(
        trigger.to_u32(),
        ActionToggleHandler { id },
    ))
}
/// A layer that maps qwerty to dvorak.
/// Don't forget to enable it, layers are off by default
pub fn dvorak<'a>() -> Box<Layer<'a>> {
    use crate::handlers::LayerAction::RewriteTo;
    use crate::key_codes::KeyCode::*;
    Box::new(Layer::new(
        vec![
            (Q, Quote),
            (W, Comma),
            (E, Dot),
            (R, P),
            (T, Y),
            (Y, F),
            (U, G),
            (I, C),
            (O, R),
            (P, L),
            //(A, (A),
            (S, O),
            (D, E),
            (F, U),
            (G, I),
            (H, D),
            (J, H),
            (K, T),
            (L, N),
            (SColon, S),
            (Quote, Minus),
            (Z, SColon),
            (X, Q),
            (C, J),
            (V, K),
            (B, X),
            (N, B),
            (M, M),
            (Comma, W),
            (Dot, V),
            (Slash, Z),
            //(BSlash, Bslash),
            (Equal, RBracket),
            (Quote, Minus),
            (RBracket, Equal),
            //(Grave, (Grave),
            (Minus, LBracket),
            (LBracket, Slash),
        ]
        .into_iter()
        .map(|(f, t)| (f, RewriteTo(t.into())))
        .collect(),
    ))
}

/// Enable/disable handler (layer) on activation/deactivation
/// for use with PressRelease, StickyKeys, OneShot, SpaceCadet
///
/// Can also be used with Modifier::* (pass in mod as HandelerID)
pub struct ActionHandler {
    id: HandlerID,
}
impl OnOff for ActionHandler {
    fn on_activate(&mut self, output: &mut impl USBKeyOut) {
        output.state().enable_handler(self.id);
    }
    fn on_deactivate(&mut self, output: &mut impl USBKeyOut) {
        output.state().disable_handler(self.id);
    }
}
/// make the shift keys behave as a OneShot
pub fn one_shot_shift(held_timeout: u16, released_timeout: u16) -> Box<OneShot<ActionHandler>> {
    Box::new(OneShot::new(
        KeyCode::LShift,
        KeyCode::RShift,
        ActionHandler {
            id: Shift as HandlerID,
        },
        held_timeout,
        released_timeout,
    ))
}
/// make the ctrl keys behave as a OneShot
pub fn one_shot_ctrl(held_timeout: u16, released_timeout: u16) -> Box<OneShot<ActionHandler>> {
    Box::new(OneShot::new(
        KeyCode::LCtrl,
        KeyCode::RCtrl,
        ActionHandler {
            id: Ctrl as HandlerID,
        },
        held_timeout,
        released_timeout,
    ))
}
/// make the alt keys behave as a OneShot
pub fn one_shot_alt(held_timeout: u16, released_timeout: u16) -> Box<OneShot<ActionHandler>> {
    Box::new(OneShot::new(
        KeyCode::LAlt,
        KeyCode::RAlt,
        ActionHandler {
            id: Alt as HandlerID,
        },
        held_timeout,
        released_timeout,
    ))
}
/// make the gui/windows key behave as a OneShot
pub fn one_shot_gui(held_timeout: u16, released_timeout: u16) -> Box<OneShot<ActionHandler>> {
    Box::new(OneShot::new(
        KeyCode::LGui,
        KeyCode::RGui,
        ActionHandler {
            id: Gui as HandlerID,
        },
        held_timeout,
        released_timeout,
    ))
}
/// Toggle a handler (layer) based on OneShot behaviour
pub fn one_shot_handler(
    trigger: impl AcceptsKeycode,
    id: HandlerID,
    held_timeout: u16,
    released_timeout: u16,
) -> Box<OneShot<ActionHandler>> {
    Box::new(OneShot::new(
        trigger,
        KeyCode::No,
        ActionHandler { id },
        held_timeout,
        released_timeout,
    ))
}
pub fn space_cadet_handler(
    trigger: impl AcceptsKeycode,
    id: HandlerID,
) -> Box<SpaceCadet<ActionHandler>> {
    Box::new(SpaceCadet::new(trigger, ActionHandler { id }))
}
/// Handler for turing Copy/Paste/Cut Keycodes into 'universal'
/// Ctrl-Insert, Shift-insert, shift-delete keystrokes
/// for dedicated copy paste keys
/// 0
pub struct CopyPaste {}
impl<T: USBKeyOut> ProcessKeys<T> for CopyPaste {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) {
        //step 0: on key release, remove all prior key presses.
        for (e, status) in iter_unhandled_mut(events) {
            match e {
                Event::KeyPress(kc) => {
                    if kc.keycode == KeyCode::Copy.into() {
                        output.send_keys(&[KeyCode::LCtrl, KeyCode::Insert]);
                        output.send_empty();
                        *status = EventStatus::Handled;
                    }
                    if kc.keycode == KeyCode::Paste.into() {
                        output.send_keys(&[KeyCode::LShift, KeyCode::Insert]);
                        output.send_empty();
                        *status = EventStatus::Handled;
                    }
                    if kc.keycode == KeyCode::Cut.into() {
                        output.send_keys(&[KeyCode::LShift, KeyCode::Delete]);
                        output.send_empty();
                        *status = EventStatus::Handled;
                    }
                }
                Event::KeyRelease(kc) => {
                    if kc.keycode == KeyCode::Copy.into() {
                        *status = EventStatus::Handled;
                    }
                    if kc.keycode == KeyCode::Paste.into() {
                        *status = EventStatus::Handled;
                    }
                    if kc.keycode == KeyCode::Cut.into() {
                        *status = EventStatus::Handled;
                    }
                }
                _ => {}
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::handlers::USBKeyboard;
    #[allow(unused_imports)]
    use crate::key_codes::KeyCode;
    #[allow(unused_imports)]
    use crate::premade::{dvorak, toggle_handler};
    #[allow(unused_imports)]
    use crate::test_helpers::{check_output, KeyOutCatcher};
    use crate::Modifier::*;
    #[allow(unused_imports)]
    use crate::{
        Event, EventStatus, Keyboard, KeyboardState, ProcessKeys, USBKeyOut, UnicodeSendMode,
    };
    use no_std_compat::prelude::v1::*;

    #[test]
    fn test_toggle_handler() {
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let id = keyboard.add_handler(Box::new(crate::handlers::UnicodeKeyboard {}));
        let tid = keyboard.add_handler(toggle_handler(0xF0100u32, id));
        assert!(keyboard.output.state().is_handler_enabled(id));
        assert!(keyboard.output.state().is_handler_enabled(tid));
        keyboard.add_keypress(0xF0100u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(!keyboard.output.state().is_handler_enabled(id));
        keyboard.add_keyrelease(0xF0100u32, 1);
        keyboard.handle_keys().unwrap();
        assert!(!keyboard.output.state().is_handler_enabled(id));
        keyboard.add_keypress(0xF0100u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().is_handler_enabled(id));
    }
    #[test]
    fn test_layer_double_rewrite_dvorak() {
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let l = dvorak();
        let layer_id = keyboard.add_handler(l);
        assert!(!keyboard.output.state().is_handler_enabled(layer_id));
        keyboard.output.state().enable_handler(layer_id);
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        keyboard.add_keypress(KeyCode::Q, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::Quote]]);
        keyboard.add_keyrelease(KeyCode::Q, 0);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[KeyCode::Quote], &[]]);
    }
    #[test]
    fn test_dvorak_brackets() {
        use crate::handlers;
        //use crate::debug_handlers;
        use crate::premade;
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let dvorak_id = keyboard.add_handler(premade::dvorak());
        keyboard.output.state().enable_handler(dvorak_id);
        keyboard.add_handler(Box::new(handlers::USBKeyboard::new()));
        keyboard.add_keypress(KeyCode::LBracket, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::Slash]]);
        keyboard.add_keyrelease(KeyCode::LBracket, 0);
        keyboard.handle_keys().unwrap();
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::RBracket, 0);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[KeyCode::Equal]]);
        keyboard.add_keyrelease(KeyCode::RBracket, 0);
        keyboard.handle_keys().unwrap();
        keyboard.output.clear();
    }
    #[test]
    fn test_oneshot_shift() {
        use crate::handlers;
        //use crate::debug_handlers;
        use crate::premade;
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(premade::one_shot_shift(0, 0));
        keyboard.add_handler(Box::new(handlers::USBKeyboard::new()));
        keyboard.add_keypress(KeyCode::RShift, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift]]); //note that the one shots always output the L variants
        keyboard.output.clear();
        assert!(keyboard.output.state().modifier(Shift));
        keyboard.add_keyrelease(KeyCode::RShift, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().modifier(Shift));
        check_output(&keyboard, &[&[KeyCode::LShift]]); //key is released, but shift is still set
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().modifier(Shift));
        check_output(&keyboard, &[&[KeyCode::LShift, KeyCode::A]]); //shift still set
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        assert!(!keyboard.output.state().modifier(Shift));
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::RShift, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift]]); //note that the one shots always output the L variants
        keyboard.output.clear();
        assert!(keyboard.output.state().modifier(Shift));
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().modifier(Shift));
        check_output(&keyboard, &[&[KeyCode::LShift, KeyCode::A]]); //key is released, but shift is still set
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().modifier(Shift));
        check_output(&keyboard, &[&[KeyCode::LShift]]);
        keyboard.output.clear();
        //we have not released the shift key!
        keyboard.add_keyrelease(KeyCode::RShift, 0);
        keyboard.handle_keys().unwrap();
        assert!(!keyboard.output.state().modifier(Shift));
        check_output(&keyboard, &[&[]]); //now we're good
        keyboard.output.clear();
    }
    #[test]
    fn test_oneshot_interaction() {
        use crate::handlers;
        //use crate::debug_handlers;
        use crate::premade;
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let dv = keyboard.add_handler(premade::dvorak());
        keyboard.add_handler(premade::one_shot_shift(0, 0));
        keyboard.add_handler(premade::one_shot_ctrl(0, 0));
        keyboard.add_handler(premade::one_shot_handler(0xF0000u32, dv, 0, 0));
        keyboard.add_handler(Box::new(handlers::USBKeyboard::new()));
        keyboard.add_keypress(KeyCode::RShift, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift]]); //note that the one shots always output the L variants
        keyboard.output.clear();
        assert!(keyboard.output.state().modifier(Shift));
        keyboard.add_keyrelease(KeyCode::RShift, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().modifier(Shift));
        check_output(&keyboard, &[&[KeyCode::LShift]]); //key is released, but shift is still set
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::RCtrl, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().modifier(Ctrl));
        assert!(keyboard.output.state().modifier(Shift));
        check_output(&keyboard, &[&[KeyCode::LShift, KeyCode::LCtrl]]); //note that the one shots always output the L variants
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::RCtrl, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().modifier(Shift));
        assert!(keyboard.output.state().modifier(Ctrl));
        check_output(&keyboard, &[&[KeyCode::LShift, KeyCode::LCtrl]]); //key is released, but shift is still set
        keyboard.output.clear();
        assert!(!keyboard.output.state().is_handler_enabled(dv));
        keyboard.add_keypress(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().modifier(Ctrl));
        assert!(keyboard.output.state().modifier(Shift));
        assert!(keyboard.output.state().is_handler_enabled(dv));
        check_output(&keyboard, &[&[KeyCode::LShift, KeyCode::LCtrl]]); //note that the one shots always output the L variants
        keyboard.output.clear();
        keyboard.add_keyrelease(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().modifier(Shift));
        assert!(keyboard.output.state().modifier(Ctrl));
        assert!(keyboard.output.state().is_handler_enabled(dv));
        check_output(&keyboard, &[&[KeyCode::LShift, KeyCode::LCtrl]]); //key is released, but shift is still set
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().modifier(Shift));
        assert!(keyboard.output.state().modifier(Ctrl));
        assert!(keyboard.output.state().is_handler_enabled(dv));
        check_output(&keyboard, &[&[KeyCode::LShift, KeyCode::LCtrl, KeyCode::Q]]); //key is released, but shift is still set
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        assert!(!keyboard.output.state().modifier(Shift));
        assert!(!keyboard.output.state().modifier(Ctrl));
        assert!(!keyboard.output.state().is_handler_enabled(dv));
        check_output(&keyboard, &[&[]]); //key is released, but shift is still set
    }
    #[test]
    fn test_oneshot_rapid_typing() {
        use crate::handlers;
        //use crate::debug_handlers;
        use crate::premade;
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(premade::one_shot_shift(0, 0));
        keyboard.add_handler(Box::new(handlers::USBKeyboard::new()));
        keyboard.add_keypress(KeyCode::RShift, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift]]); //note that the one shots always output the L variants
        keyboard.output.clear();
        assert!(keyboard.output.state().modifier(Shift));
        keyboard.add_keyrelease(KeyCode::RShift, 50);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().modifier(Shift));
        check_output(&keyboard, &[&[KeyCode::LShift]]); //key is released, but shift is still set
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::A, 50);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().modifier(Shift));
        check_output(&keyboard, &[&[KeyCode::LShift, KeyCode::A]]); //key is released, but shift is still set
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::B, 50);
        keyboard.handle_keys().unwrap();
        assert!(!keyboard.output.state().modifier(Shift));
        check_output(&keyboard, &[&[KeyCode::A, KeyCode::B]]); //key is released, but shift is still set
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::A, 50);
        keyboard.handle_keys().unwrap();
        assert!(!keyboard.output.state().modifier(Shift));
        check_output(&keyboard, &[&[KeyCode::B]]); //key is released, but shift is still set
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::A, 50);
        keyboard.handle_keys().unwrap();
        assert!(!keyboard.output.state().modifier(Shift));
        check_output(&keyboard, &[&[KeyCode::B]]); //key is released, but shift is still set
        keyboard.output.clear();
    }
    #[test]
    fn test_oneshot_released_timeout() {
        use crate::handlers;
        //use crate::debug_handlers;
        use crate::premade;

        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(premade::one_shot_shift(0, 1000));
        keyboard.add_handler(Box::new(handlers::USBKeyboard::new()));

        keyboard.add_keypress(KeyCode::RShift, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift]]); //note that the one shots always output the L variants
        keyboard.output.clear();
        assert!(keyboard.output.state().modifier(Shift));

        keyboard.add_timeout(1000);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift]]); //note that the one shots always output the L variants
        keyboard.output.clear();
        assert!(keyboard.output.state().modifier(Shift));

        keyboard.add_keyrelease(KeyCode::RShift, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift]]); //note that the one shots always output the L variants
        keyboard.output.clear();

        keyboard.add_timeout(1000);
        keyboard.handle_keys().unwrap();
        assert!(!keyboard.output.state().modifier(Shift));
        check_output(&keyboard, &[&[]]); //note that the one shots always output the L variants
    }
}
