use crate::handlers::RewriteLayer;
/// premade handlers for various occacions
use crate::handlers::{Action, OnOff, OneShot, PressReleaseMacro, SpaceCadet, HandlerResult, ProcessKeys};
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use crate::Modifier::*;
use crate::{AcceptsKeycode, HandlerID, KeyCode, USBKeyOut};
use no_std_compat::prelude::v1::*;
///toggle a handler on activate
/// do noting on deactivate
/// probably only usefull with PressReleaseMacro
/// used by toggle_handler()
pub struct ActionToggleHandler {
    pub id: HandlerID,
}
impl OnOff for ActionToggleHandler {
    fn on_activate(&mut self, output: &mut dyn USBKeyOut) {
        output.state().toggle_handler(self.id);
    }
    fn on_deactivate(&mut self, _output: &mut dyn USBKeyOut) {}
}

impl Action for ActionToggleHandler {
    fn on_trigger(&mut self, output: &mut dyn USBKeyOut) {
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
pub fn dvorak() -> Box<RewriteLayer> {
    use crate::key_codes::KeyCode::*;
    const MAP: &[(u32, u32)] = &[
        (Q.to_u32(), Quote.to_u32()),
        (W.to_u32(), Comma.to_u32()),
        (E.to_u32(), Dot.to_u32()),
        (R.to_u32(), P.to_u32()),
        (T.to_u32(), Y.to_u32()),
        (Y.to_u32(), F.to_u32()),
        (U.to_u32(), G.to_u32()),
        (I.to_u32(), C.to_u32()),
        (O.to_u32(), R.to_u32()),
        (P.to_u32(), L.to_u32()),
        //(A.to_u32(), (A.to_u32()),
        (S.to_u32(), O.to_u32()),
        (D.to_u32(), E.to_u32()),
        (F.to_u32(), U.to_u32()),
        (G.to_u32(), I.to_u32()),
        (H.to_u32(), D.to_u32()),
        (J.to_u32(), H.to_u32()),
        (K.to_u32(), T.to_u32()),
        (L.to_u32(), N.to_u32()),
        (SColon.to_u32(), S.to_u32()),
        (Quote.to_u32(), Minus.to_u32()),
        (Z.to_u32(), SColon.to_u32()),
        (X.to_u32(), Q.to_u32()),
        (C.to_u32(), J.to_u32()),
        (V.to_u32(), K.to_u32()),
        (B.to_u32(), X.to_u32()),
        (N.to_u32(), B.to_u32()),
        (M.to_u32(), M.to_u32()),
        (Comma.to_u32(), W.to_u32()),
        (Dot.to_u32(), V.to_u32()),
        (Slash.to_u32(), Z.to_u32()),
        //(BSlash.to_u32(), Bslash.to_u32()),
        (Equal.to_u32(), RBracket.to_u32()),
        (Quote.to_u32(), Minus.to_u32()),
        (RBracket.to_u32(), Equal.to_u32()),
        //(Grave.to_u32(), (Grave.to_u32()),
        (Minus.to_u32(), LBracket.to_u32()),
        (LBracket.to_u32(), Slash.to_u32()),
    ];
    Box::new(RewriteLayer::new(MAP))
}

/// Enable/disable handler (layer) on activation/deactivation
/// for use with PressRelease, StickyKeys, OneShot, SpaceCadet
///
/// Can also be used with Modifier::* (pass in mod as HandelerID)
pub struct ActionHandler {
    id: HandlerID,
}
impl ActionHandler {
    pub fn new(id: HandlerID) -> ActionHandler {
        ActionHandler{id}
    }
}
impl OnOff for ActionHandler {
    fn on_activate(&mut self, output: &mut dyn USBKeyOut) {
        output.state().enable_handler(self.id);
    }
    fn on_deactivate(&mut self, output: &mut dyn USBKeyOut) {
        output.state().disable_handler(self.id);
    }
}

/// Disable/enable handler (layer) on activation/deactivation
/// for use with PressRelease, StickyKeys, OneShot, SpaceCadet
///
/// Can also be used with Modifier::* (pass in mod as HandelerID)
/// Acts as the inverse of ActionHandler - this one enables when the button is released!
pub struct InverseActionHandler {
    id: HandlerID,
}
impl InverseActionHandler {
    pub fn new(id: HandlerID) -> InverseActionHandler {
        InverseActionHandler{id}
    }
}
impl OnOff for InverseActionHandler {
    fn on_activate(&mut self, output: &mut dyn USBKeyOut) {
        output.debug("off");
        output.state().disable_handler(self.id);
    }
    fn on_deactivate(&mut self, output: &mut dyn USBKeyOut) {
        output.debug("on");
        output.state().enable_handler(self.id);
    }
}




/// make the shift keys behave as a OneShot
/// 
/// hint: use before space cadet
pub fn one_shot_shift(held_timeout: u16, released_timeout: u16) -> Box<OneShot<ActionHandler, ActionNone, ActionNone>> {
    Box::new(OneShot::new(
        KeyCode::LShift,
        KeyCode::RShift,
        ActionHandler {
            id: Shift as HandlerID,
        },
        ActionNone{},
        ActionNone{},
        held_timeout,
        released_timeout,
    ))
}

/// make the ctrl keys behave as a OneShot
/// 
/// hint: use before space cadet
pub fn one_shot_ctrl(held_timeout: u16, released_timeout: u16) -> Box<OneShot<ActionHandler, ActionNone, ActionNone>> {
    Box::new(OneShot::new(
        KeyCode::LCtrl,
        KeyCode::RCtrl,
        ActionHandler {
            id: Ctrl as HandlerID,
        },
        ActionNone{},
        ActionNone{},
        held_timeout,
        released_timeout,
    ))
}
/// make the alt keys behave as a OneShot
/// 
/// hint: use before space cadet
pub fn one_shot_alt(held_timeout: u16, released_timeout: u16) -> Box<OneShot<ActionHandler, ActionNone, ActionNone>> {
    Box::new(OneShot::new(
        KeyCode::LAlt,
        KeyCode::RAlt,
        ActionHandler {
            id: Alt as HandlerID,
        },
        ActionNone{},
        ActionNone{},
        held_timeout,
        released_timeout,
    ))
}
/// make the gui/windows key behave as a OneShot
/// 
/// hint: use before space cadet
pub fn one_shot_gui(held_timeout: u16, released_timeout: u16) -> Box<OneShot<ActionHandler, ActionNone, ActionNone>> {
    Box::new(OneShot::new(
        KeyCode::LGui,
        KeyCode::RGui,
        ActionHandler {
            id: Gui as HandlerID,
        },
        ActionNone{},
        ActionNone{},
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
) -> Box<OneShot<ActionHandler, ActionNone, ActionNone>> {
    Box::new(OneShot::new(
        trigger,
        KeyCode::No,
        ActionHandler { id },
        ActionNone{},
        ActionNone{},
        held_timeout,
        released_timeout,
    ))
}

/// A space cadet (pass through on tap,
/// on/off on pressed+other keys)
/// that turns a handler on/off.
///
/// Note that this needs to be before the handler
/// it toggles in the handler order,
/// so you need to use
///
/// keyboard.add_handler(space_cadet_handler(trigger, keyboard.future_handler_id(2)));
/// keyboard.add_handler(Box::new(Layer(...)))
///
pub fn space_cadet_handler(
    trigger: impl AcceptsKeycode,
    action: KeyCode,
    id: HandlerID,
) -> Box<SpaceCadet<KeyCode, ActionHandler>> {
    Box::new(SpaceCadet::new(trigger, action, ActionHandler { id }))
}
/// Handler for turing Copy/Paste/Cut Keycodes into 'universal'
/// Ctrl-Insert, Shift-insert, shift-delete keystrokes
/// for dedicated copy paste keys
/// 0
pub struct CopyPaste {}
impl<T: USBKeyOut> ProcessKeys<T> for CopyPaste {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) ->HandlerResult {
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
        HandlerResult::NoOp
    }
}


/// Abort all event handling, throw away remaining events,
/// unset all modifiers and enable/disable handers as requested
/// by handler_overwrite
pub struct ActionNone;
impl Action for ActionNone{
    fn on_trigger(&mut self, _output: &mut dyn USBKeyOut) {}
}

pub struct ActionAbort {
    handler_overwrite: Vec<(HandlerID, bool)>
}

impl ActionAbort {
    pub fn new() -> ActionAbort {
        ActionAbort{handler_overwrite: Vec::new()}
    }

    pub fn set_abort_status(&mut self, handler_id: HandlerID, enabled: bool) {
        self.handler_overwrite.push((handler_id, enabled));
    }

    fn do_abort(&mut self, output: &mut dyn USBKeyOut) {
        let state = output.state();
        for (handler_id, enabled) in self.handler_overwrite.iter() {
            state.set_handler(*handler_id, *enabled);
        }
        state.set_modifier(Shift, false);
        state.set_modifier(Ctrl, false);
        state.set_modifier(Alt, false);
        state.set_modifier(Gui, false);
        state.abort_and_clear_events();
    }

}

impl Action for ActionAbort {
    fn on_trigger(&mut self, output: &mut dyn USBKeyOut) {self.do_abort(output);}
    }

impl OnOff for ActionAbort {
    fn on_activate(&mut self, output: &mut dyn USBKeyOut) {
        self.do_abort(output);
    }
    fn on_deactivate(&mut self, _output: &mut dyn USBKeyOut) {}

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

    #[test]
    fn test_abort() {
        use crate::premade;
        use crate::handlers::{RewriteLayer, PressReleaseMacro};
        use crate::UserKey;
        use crate::test_helpers::{Checks, check_output};
        use crate::Modifier::Shift;
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let mut aa = premade::ActionAbort::new();
        let should_enable = keyboard.add_handler(premade::one_shot_alt(0,0));
        const MAP: &[(u32, u32)] = &[(KeyCode::A.to_u32(), KeyCode::X.to_u32())];
        let should_disable = keyboard.add_handler(Box::new(RewriteLayer::new(&MAP)));
        keyboard.output.state().enable_handler(should_disable);
        keyboard.output.state().disable_handler(should_enable);

        aa.set_abort_status(should_enable, true);
        aa.set_abort_status(should_disable, false);

        keyboard.add_handler(
            Box::new(PressReleaseMacro::new(UserKey::UK0, aa))
        );
        keyboard.add_handler(Box::new(crate::handlers::USBKeyboard {}));

        assert!(!keyboard.output.state().is_handler_enabled(should_enable));
        assert!(keyboard.output.state().is_handler_enabled(should_disable));

        keyboard.pc(KeyCode::LShift, &[&[KeyCode::LShift]]);
        assert!(keyboard.output.state().modifier(Shift));
        keyboard.add_keypress(UserKey::UK0, 50);
        keyboard.add_keypress(KeyCode::A, 50);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[]);
 
        assert!(keyboard.output.state().is_handler_enabled(should_enable));
        assert!(!keyboard.output.state().is_handler_enabled(should_disable));

        assert!(keyboard.events.is_empty());
    }

}
