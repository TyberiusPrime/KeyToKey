///premade handlers for various occacions
use crate::handlers::{MacroCallback, PressReleaseMacro};
use crate::{AcceptsKeycode, HandlerID, USBKeyOut};
use no_std_compat::prelude::v1::*;

//Internal type for toggle_handler 
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
    Box::new(
    PressReleaseMacro::new(trigger.to_u32(), ToggleHandlerCB { id })
    )
}




#[cfg(test)]
mod test {
    use no_std_compat::prelude::v1::*;
    #[allow(unused_imports)]
    use crate::key_codes::KeyCode;
    #[allow(unused_imports)]
    use crate::test_helpers::{check_output, KeyOutCatcher};

    use crate::handlers::{
        USBKeyboard, UnicodeKeyboard,
    };
    #[allow(unused_imports)]
    use crate::{
        Event, EventStatus, Keyboard, KeyboardState, ProcessKeys, USBKeyOut, UnicodeSendMode,
    };
    #[allow(unused_imports)]
    use crate::premade::toggle_handler;
 
#[test]
    fn test_toggle_handler(){
      let mut keyboard = Keyboard::new(KeyOutCatcher::new());
      let id = keyboard.add_handler(Box::new(crate::handlers::UnicodeKeyboard{}));
      let tid = keyboard.add_handler(toggle_handler(999, id));
      assert!(keyboard.output.state().is_handler_enabled(id));
      assert!(keyboard.output.state().is_handler_enabled(tid));
      keyboard.add_keypress(999, 0);
      keyboard.handle_keys().unwrap();
      assert!(!keyboard.output.state().is_handler_enabled(id));
      keyboard.add_keyrelease(999,1);
      keyboard.handle_keys().unwrap();
      assert!(!keyboard.output.state().is_handler_enabled(id));
      keyboard.add_keypress(999, 0);
      keyboard.handle_keys().unwrap();
      assert!(keyboard.output.state().is_handler_enabled(id));
}
}