/// premade handlers for various occacions
use crate::handlers::{Layer, MacroCallback, OneShot, PressReleaseMacro};
use crate::{AcceptsKeycode, HandlerID, KeyCode, USBKeyOut};
use no_std_compat::prelude::v1::*;

/// Internal type for toggle_handler
pub struct ToggleHandlerCB {
    id: HandlerID,
}

impl MacroCallback for ToggleHandlerCB {
    fn on_activate(&mut self, output: &mut impl USBKeyOut) {
        output.state().toggle_handler(self.id);
    }
    fn on_deactivate(&mut self, _output: &mut impl USBKeyOut) {}
}

/// Toggles a handler on and off when a key is pressed
pub fn toggle_handler(
    trigger: impl AcceptsKeycode,
    id: HandlerID,
) -> Box<PressReleaseMacro<ToggleHandlerCB>> {
    Box::new(PressReleaseMacro::new(
        trigger.to_u32(),
        ToggleHandlerCB { id },
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

/// Internal type for one_shot_shift
pub struct OneShotShift {}

impl MacroCallback for OneShotShift {
    fn on_activate(&mut self, output: &mut impl USBKeyOut) {
        output.state().shift = true;
    }
    fn on_deactivate(&mut self, output: &mut impl USBKeyOut) {
        output.state().shift = false;
    }
}

/// make the shift keys behave as a OneShot
pub fn one_shot_shift() -> Box<OneShot<OneShotShift>> {
    Box::new(OneShot::new(KeyCode::LShift, KeyCode::RShift, OneShotShift {}))
}

/// Internal type for one_shot_ctrl
pub struct OneShotCtrl {}

impl MacroCallback for OneShotCtrl {
    fn on_activate(&mut self, output: &mut impl USBKeyOut) {
        output.state().ctrl = true;
    }
    fn on_deactivate(&mut self, output: &mut impl USBKeyOut) {
        output.state().ctrl = false;
    }
}

/// make the ctrl keys behave as a OneShot
pub fn one_shot_ctrl() -> Box<OneShot<OneShotCtrl>> {
    Box::new(OneShot::new(KeyCode::LCtrl, KeyCode::RCtrl, OneShotCtrl {}))
}

/// Internal type for one_shot_alt
pub struct OneShotAlt {}

impl MacroCallback for OneShotAlt {
    fn on_activate(&mut self, output: &mut impl USBKeyOut) {
        output.state().alt = true;
    }
    fn on_deactivate(&mut self, output: &mut impl USBKeyOut) {
        output.state().alt = false;
    }
}

/// make the alt keys behave as a OneShot
pub fn one_shot_alt() -> Box<OneShot<OneShotAlt>> {
    Box::new(OneShot::new(KeyCode::LAlt, KeyCode::RAlt, OneShotAlt {}))
}


/// Internal type for one_shot_gui
pub struct OneShotGui {}

impl MacroCallback for OneShotGui {
    fn on_activate(&mut self, output: &mut impl USBKeyOut) {
        output.state().gui = true;
    }
    fn on_deactivate(&mut self, output: &mut impl USBKeyOut) {
        output.state().gui = false;
    }
}

/// make the gui/windows key behave as a OneShot
pub fn one_shot_gui() -> Box<OneShot<OneShotGui>> {
    Box::new(OneShot::new(KeyCode::LGui, KeyCode::RGui, OneShotGui {}))
}

/// Internal type for one_shot_handler
pub struct OneShotHandler {
    id: HandlerID
}

impl MacroCallback for OneShotHandler {
    fn on_activate(&mut self, output: &mut impl USBKeyOut) {
        output.state().enable_handler(self.id);
    }
    fn on_deactivate(&mut self, output: &mut impl USBKeyOut) {
        output.state().disable_handler(self.id);
    }
}

/// Toggle a handler (layer) based on OneShot behaviour
pub fn one_shot_handler(trigger: impl AcceptsKeycode, id: HandlerID) -> Box<OneShot<OneShotHandler>> {
    Box::new(OneShot::new(trigger, KeyCode::No, OneShotHandler {id}))
}



#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use crate::key_codes::KeyCode;
    #[allow(unused_imports)]
    use crate::test_helpers::{check_output, KeyOutCatcher};
    use no_std_compat::prelude::v1::*;

    use crate::handlers::USBKeyboard;
    #[allow(unused_imports)]
    use crate::premade::{dvorak, toggle_handler};
    #[allow(unused_imports)]
    use crate::{
        Event, EventStatus, Keyboard, KeyboardState, ProcessKeys, USBKeyOut, UnicodeSendMode,
    };

    #[test]
    fn test_toggle_handler() {
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let id = keyboard.add_handler(Box::new(crate::handlers::UnicodeKeyboard {}));
        let tid = keyboard.add_handler(toggle_handler(999, id));
        assert!(keyboard.output.state().is_handler_enabled(id));
        assert!(keyboard.output.state().is_handler_enabled(tid));
        keyboard.add_keypress(999, 0);
        keyboard.handle_keys().unwrap();
        assert!(!keyboard.output.state().is_handler_enabled(id));
        keyboard.add_keyrelease(999, 1);
        keyboard.handle_keys().unwrap();
        assert!(!keyboard.output.state().is_handler_enabled(id));
        keyboard.add_keypress(999, 0);
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
        keyboard.add_handler(premade::one_shot_shift());
        keyboard.add_handler(Box::new(handlers::USBKeyboard::new()));

        keyboard.add_keypress(KeyCode::RShift, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift]]); //note that the one shots always output the L variants
        keyboard.output.clear();
        assert!(keyboard.output.state().shift);

        keyboard.add_keyrelease(KeyCode::RShift, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().shift);
        check_output(&keyboard, &[&[KeyCode::LShift]]); //key is released, but shift is still set
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().shift);
        check_output(&keyboard, &[&[KeyCode::LShift, KeyCode::A]]); //key is released, but shift is still set
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        assert!(!keyboard.output.state().shift);
        check_output(&keyboard, &[&[]]); //key is released, but shift is still set
    }

    #[test]
    fn test_oneshot_interaction() {
       use crate::handlers;
        //use crate::debug_handlers;
        use crate::premade;
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let dv = keyboard.add_handler(premade::dvorak());
        keyboard.add_handler(premade::one_shot_shift());
        keyboard.add_handler(premade::one_shot_ctrl());
        keyboard.add_handler(premade::one_shot_handler(0xF0000u32, dv));
        keyboard.add_handler(Box::new(handlers::USBKeyboard::new()));

        keyboard.add_keypress(KeyCode::RShift, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift]]); //note that the one shots always output the L variants
        keyboard.output.clear();
        assert!(keyboard.output.state().shift);

        keyboard.add_keyrelease(KeyCode::RShift, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().shift);
        check_output(&keyboard, &[&[KeyCode::LShift]]); //key is released, but shift is still set
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::RCtrl, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().ctrl);
        assert!(keyboard.output.state().shift);
        check_output(&keyboard, &[&[KeyCode::LShift, KeyCode::LCtrl]]); //note that the one shots always output the L variants
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::RCtrl, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().shift);
        assert!(keyboard.output.state().ctrl);
        check_output(&keyboard, &[&[KeyCode::LShift, KeyCode::LCtrl]]); //key is released, but shift is still set
        keyboard.output.clear();

        assert!(!keyboard.output.state().is_handler_enabled(dv));
        keyboard.add_keypress(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().ctrl);
        assert!(keyboard.output.state().shift);
        assert!(keyboard.output.state().is_handler_enabled(dv));
        check_output(&keyboard, &[&[KeyCode::LShift, KeyCode::LCtrl]]); //note that the one shots always output the L variants
        keyboard.output.clear();

        keyboard.add_keyrelease(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().shift);
        assert!(keyboard.output.state().ctrl);
        assert!(keyboard.output.state().is_handler_enabled(dv));
        check_output(&keyboard, &[&[KeyCode::LShift, KeyCode::LCtrl]]); //key is released, but shift is still set
        keyboard.output.clear();



        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.state().shift);
        assert!(keyboard.output.state().ctrl);
        assert!(keyboard.output.state().is_handler_enabled(dv));
        check_output(&keyboard, &[&[KeyCode::LShift, KeyCode::LCtrl, KeyCode::Q]]); //key is released, but shift is still set
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        assert!(!keyboard.output.state().shift);
        assert!(!keyboard.output.state().ctrl);
        assert!(!keyboard.output.state().is_handler_enabled(dv));

        check_output(&keyboard, &[&[]]); //key is released, but shift is still set
    }


}
