use no_std_compat::prelude::v1::*;
use crate::key_stream::{Event, EventStatus};

mod leader;
mod oneshot;
mod unicodekeyboard;
mod usbkeyboard;
mod layer;
mod macros;
mod tapdance;
mod spacecadet;
mod autoshift;

pub use leader::{Leader};
pub use oneshot::OneShot;
pub use unicodekeyboard::UnicodeKeyboard;
pub use usbkeyboard::USBKeyboard;
pub use layer::{Layer, LayerAction};
pub use macros::{PressReleaseMacro, StickyMacro};
pub use tapdance::TapDance;
pub use spacecadet::SpaceCadet;
pub use autoshift::AutoShift;

use crate::USBKeyOut;


/// Handlers are defined by this trait
///
/// they process the events, set their status to either Handled or Ignored
/// (if more data is necessary), and send input to the computer via output
pub trait ProcessKeys<T: USBKeyOut> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> ();
    /// whether this handler is enabled after add_handlers
    /// (true for most, false for Layers)
    fn default_enabled(&self) -> bool {
        true
    }
}


/// A trait for macro callbacks
/// 
/// see PressReleaseMacro, StickyMacro
pub trait MacroCallback {
    fn on_activate(&mut self, output: &mut impl USBKeyOut);
    fn on_deactivate(&mut self, output: &mut impl USBKeyOut);
}

/// an Action
/// 
/// For example by a leader sequence or a tap dance.
/// Contrast with LayerAction which is a superset of Action
/// 
/// Notably implemented on &str, so you can just pass in a &str 
/// to send as the action!
trait NonLayerAction<T: USBKeyOut> {
    fn leader_sequence_accepted(&mut self, output: &mut T); 
    }

impl <T: USBKeyOut> NonLayerAction<T> for &str {
    fn leader_sequence_accepted(&mut self, output: &mut T){ 
        output.send_string(self);
    }

}



