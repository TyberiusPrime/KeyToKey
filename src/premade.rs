/// premade handlers for various occacions
use crate::handlers::{Layer, MacroCallback, PressReleaseMacro};
use crate::{AcceptsKeycode, HandlerID, USBKeyOut};
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
        use crate::debug_handlers;
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
}
