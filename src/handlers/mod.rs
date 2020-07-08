use crate::key_codes::KeyCode;
use crate::{Event, EventStatus};
use no_std_compat::prelude::v1::*;

mod autoshift;
mod layer;
mod leader;
mod longtap;
mod macros;
mod oneshot;
mod rewrite_layer;
mod sequence;
mod spacecadet;
mod tapdance;
mod unicodekeyboard;
mod usbkeyboard;
pub mod debug_handlers;

use crate::USBKeyOut;
pub use autoshift::AutoShift;
pub use layer::{Layer, LayerAction, AutoOff};
pub use rewrite_layer::RewriteLayer;
//pub use leader::Leader;
pub use longtap::LongTap;
pub use macros::{PressMacro, PressReleaseMacro, StickyMacro};
pub use oneshot::OneShot;
pub use sequence::Sequence;
pub use spacecadet::SpaceCadet;
pub use tapdance::{TapDance, TapDanceAction, TapDanceEnd};
pub use unicodekeyboard::UnicodeKeyboard;
pub use usbkeyboard::USBKeyboard;
/// Handlers are defined by this trait
///
/// they process the events, set their status to either Handled or Ignored
/// (if more data is necessary), and send input to the computer via output
pub trait ProcessKeys<T: USBKeyOut> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> HandlerResult;
    /// whether this handler is enabled after add_handlers
    /// (true for most, false for Layers)
    fn default_enabled(&self) -> bool {
        true
    }
}

pub enum HandlerResult {
    NoOp,
    Disable 
}


/// A callback used when one single action is needed
///
/// examples: Leader invocations.
///
/// Notably implemented on &str, so you can just pass in a &str
/// to be send to the host computer.

pub trait Action: Send+ Sync {
    fn on_trigger(&mut self, output: &mut dyn USBKeyOut);
}

/// send a string as an Action
impl Action for &str {
    fn on_trigger(&mut self, output: &mut dyn USBKeyOut) {
        output.send_string(self);
    }
}

/// Register a key as an Action
///
/// that means the current modifiers are sent as well by USBKeyboard
impl Action for KeyCode {
    fn on_trigger(&mut self, output: &mut dyn USBKeyOut) {
        output.register_key(*self);
    }
}

///Register multiple key codes as OnOff action
impl Action for Vec<KeyCode> {
    fn on_trigger(&mut self, output: &mut dyn USBKeyOut) {
        output.send_keys(self);
        output.send_empty();
    }
}


/// A trait for callbacks when an on/off action is needed
///
///
/// Used by PressReleaseMacros, StickyMacros, OneShots
/// see PressReleaseMacro, StickyMacro
pub trait OnOff {
    fn on_activate(&mut self, output: &mut dyn USBKeyOut);
    fn on_deactivate(&mut self, output: &mut dyn USBKeyOut);
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
impl<T: USBKeyOut> NonLayerAction<T> for &str {
    fn leader_sequence_accepted(&mut self, output: &mut T) {
        output.send_string(self);
    }
}
