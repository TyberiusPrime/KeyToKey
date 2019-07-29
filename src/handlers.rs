use crate::key_codes::{AcceptsKeycode, KeyCode};
use crate::key_stream::{iter_unhandled_mut, Event, EventStatus};
use core::convert::TryInto;
use no_std_compat::prelude::v1::*;
use smallbitvec::sbvec;

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

/// The default bottom layer
///
/// this simulates a bog standard regular USB
/// Keyboard.
/// Just map your keys to the usb keycodes.
///
/// key repeat is whatever usb does...
pub struct USBKeyboard {}

impl USBKeyboard {
    pub fn new() -> USBKeyboard {
        USBKeyboard {}
    }
}

impl<T: USBKeyOut> ProcessKeys<T> for USBKeyboard {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
        //step 0: on key release, remove all prior key presses.
        let mut codes_to_delete: Vec<u32> = Vec::new();
        let mut modifiers_sent = sbvec![false; 4];
        for (e, status) in iter_unhandled_mut(events).rev() {
            //note that we're doing this in reverse, ie. releases happen before presses.
            match e {
                Event::KeyRelease(kc) => {
                    if kc.keycode < 256 {
                        if !codes_to_delete.contains(&kc.keycode) {
                            codes_to_delete.push(kc.keycode);
                        }
                        *status = EventStatus::Handled;
                    }
                    if kc.keycode == KeyCode::LShift.into() || kc.keycode == KeyCode::RShift.into()
                    {
                        output.state().shift = false;
                    } else if kc.keycode == KeyCode::LAlt.into()
                        || kc.keycode == KeyCode::RAlt.into()
                    {
                        output.state().alt = false;
                    } else if kc.keycode == KeyCode::LCtrl.into()
                        || kc.keycode == KeyCode::RCtrl.into()
                    {
                        output.state().ctrl = false;
                    } else if kc.keycode == KeyCode::LGui.into()
                        || kc.keycode == KeyCode::RGui.into()
                    {
                        output.state().meta = false;
                    }
                }
                Event::KeyPress(kc) => {
                    let mut send = false;
                    if codes_to_delete.contains(&kc.keycode) {
                        *status = EventStatus::Handled;
                        if kc.flag == 0 {
                            //we have never send this before
                            send = true;
                        }
                    } else {
                        send = true;
                        if kc.keycode == KeyCode::LShift.into()
                            || kc.keycode == KeyCode::RShift.into()
                        {
                            output.state().shift = true;
                            modifiers_sent.set(0, true);
                        } else if kc.keycode == KeyCode::LAlt.into()
                            || kc.keycode == KeyCode::RAlt.into()
                        {
                            output.state().alt = true;
                            modifiers_sent.set(1, true);
                        } else if kc.keycode == KeyCode::LCtrl.into()
                            || kc.keycode == KeyCode::RCtrl.into()
                        {
                            output.state().ctrl = true;
                            modifiers_sent.set(2, true);
                        } else if kc.keycode == KeyCode::LGui.into()
                            || kc.keycode == KeyCode::RGui.into()
                        {
                            output.state().meta = true;
                            modifiers_sent.set(3, true);
                        }
                    }
                    if kc.keycode < 256 {
                        let oc: Result<KeyCode, String> = (kc.keycode as u8).try_into();
                        match oc {
                            Ok(x) => {
                                if send {
                                    output.register_key(x);
                                }
                                if *status != EventStatus::Handled {
                                    *status = EventStatus::Ignored; //so we may resend it...
                                }
                            }
                            Err(_) => *status = EventStatus::Handled, //throw it away, will ya?
                        };
                        kc.flag = 1;
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
        if output.state().shift && !modifiers_sent[0] {
            output.register_key(KeyCode::LShift);
        }
        if output.state().alt && !modifiers_sent[1] {
            output.register_key(KeyCode::LAlt);
        }
        if output.state().ctrl && !modifiers_sent[2] {
            output.register_key(KeyCode::LCtrl);
        }
        if output.state().meta && !modifiers_sent[3] {
            output.register_key(KeyCode::LGui);
        }

        output.send_registered();
    }
}

/// This processor sends unicode 'characters'
/// just map your keys to unicode 'characters' (if code > 256)
/// for unicode codes < 256 use unicode code + UNICODE_BELOW_256
/// (=0x100000).
/// sending happens on keyrelease - no key repeat
///
/// use the range 0xF0000..=0xFFFFD, or 0x1000FF..=0x10FFFD
///  for custom key codes that are note processed
pub struct UnicodeKeyboard {}
impl UnicodeKeyboard {
    fn is_unicode_keycode(keycode: u32) -> bool {
        match keycode {
            0..=0xFF => false,            //these we ignore
            0xF0000..=0xFFFFD => false,   //unicode private character range A
            0x1000FF..=0x10FFFD => false, //unicode private character range b (minus those we use for codes < 256)
            _ => true,
        }
    }
    fn keycode_to_unicode(keycode: u32) -> u32 {
        if keycode < 0x100000 {
            keycode
        } else {
            keycode - 0x100000
        }
    }
}
impl<T: USBKeyOut> ProcessKeys<T> for UnicodeKeyboard {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
        for (event, status) in iter_unhandled_mut(events) {
            match event {
                Event::KeyPress(kc) => {
                    if UnicodeKeyboard::is_unicode_keycode(kc.keycode) {
                        *status = EventStatus::Handled;
                    }
                }
                Event::KeyRelease(kc) => {
                    if UnicodeKeyboard::is_unicode_keycode(kc.keycode) {
                        let c = no_std_compat::char::from_u32(UnicodeKeyboard::keycode_to_unicode(
                            kc.keycode,
                        ));
                        if let Some(c) = c {
                            output.send_unicode(c);
                        }
                        *status = EventStatus::Handled;
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
    }
}

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

pub enum LayerAction<'a> {
    RewriteTo(u32),
    RewriteToShifted(u32, u32),
    //todo: rewrite shift
    SendString(&'a str),
    //    Callback(fn(&mut T) -> (), fn(&mut T) -> ()),
}
pub struct Layer<'a> {
    rewrites: Vec<(u32, LayerAction<'a>)>,
}

impl Layer<'_> {
    pub fn new<F: AcceptsKeycode>(rewrites: Vec<(F, LayerAction)>) -> Layer<'_> {
        Layer {
            rewrites: rewrites
                .into_iter()
                .map(|(trigger, action)| (trigger.to_u32(), action))
                .collect(),
        }
    }
}

impl<T: USBKeyOut> ProcessKeys<T> for Layer<'_> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
        for (event, status) in events.iter_mut() {
            match event {
                Event::KeyRelease(kc) => {
                    for (from, to) in self.rewrites.iter() {
                        if *from == kc.keycode {
                            match to {
                                LayerAction::RewriteTo(to_keycode) => {
                                    kc.keycode = *to_keycode;
                                }
                                LayerAction::RewriteToShifted(to_keycode, to_shifted_keycode) => {
                                    if output.state().shift {
                                        kc.keycode = *to_shifted_keycode;
                                    } else {
                                        kc.keycode = *to_keycode;
                                    }
                                }
                                LayerAction::SendString(s) => {
                                    output.send_string(s);
                                    *status = EventStatus::Handled;
                                }
                            }
                        }
                    }
                }
                Event::KeyPress(kc) => {
                    for (from, to) in self.rewrites.iter() {
                        if *from == kc.keycode {
                            match to {
                                LayerAction::RewriteTo(to_keycode) => {
                                    kc.keycode = *to_keycode;
                                }
                                LayerAction::RewriteToShifted(to_keycode, to_shifted_keycode) => {
                                    if output.state().shift {
                                        kc.keycode = *to_shifted_keycode;
                                    } else {
                                        kc.keycode = *to_keycode;
                                    }
                                }
                                LayerAction::SendString(_) => *status = EventStatus::Handled,
                            }
                        }
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
    }

    fn default_enabled(&self) -> bool {
        false
    }
}

/// The simples callback -
/// call on_press(output: impl USBKeyOut) on key press
/// and on_release(output) on release))
/// trigger may be any keycode,
/// but preferentialy from the region 0xF00FF..=0xFFFFD
/// which is not used by either UnicodeKeyboard or UsbKeyboard
pub struct PressReleaseMacro<'a, T, F1, F2> {
    keycode: u32,
    on_press: Option<F1>,
    on_release: Option<F2>,
    phantom: core::marker::PhantomData<&'a T>,
}
impl<'a, T: USBKeyOut, F1: FnMut(&mut T), F2: FnMut(&mut T)> PressReleaseMacro<'a, T, F1, F2> {
    pub fn new(
        trigger: impl AcceptsKeycode,
        on_press: Option<F1>,
        on_release: Option<F2>,
    ) -> PressReleaseMacro<'a, T, F1, F2> {
        PressReleaseMacro {
            keycode: trigger.to_u32(),
            on_press,
            on_release,
            phantom: core::marker::PhantomData,
        }
    }
}

impl<T: USBKeyOut, F1: FnMut(&mut T), F2: FnMut(&mut T)> ProcessKeys<T>
    for PressReleaseMacro<'_, T, F1, F2>
{
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
        for (event, status) in events.iter_mut() {
            match event {
                Event::KeyPress(kc) => {
                    if kc.keycode == self.keycode {
                        *status = EventStatus::Handled;
                        match &mut self.on_press {
                            Some(x) => (*x)(output),
                            None => {}
                        }
                    }
                }
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.keycode {
                        *status = EventStatus::Handled;
                        match &mut self.on_release {
                            Some(x) => (*x)(output),
                            None => {}
                        }
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
    }
}

/// a macro that is called 'on' on the the first keypress
/// and off on the second keyrelease.
/// Using this you can implement e.g. sticky modifiers
///
pub struct StickyMacro<'a, T, F1: FnMut(&mut T), F2: FnMut(&mut T)> {
    keycode: u32,
    on_toggle_on: F1,
    on_toggle_off: F2,
    active: u8,
    phantom: core::marker::PhantomData<&'a T>,
}
impl<'a, T: USBKeyOut, F1: FnMut(&mut T), F2: FnMut(&mut T)> StickyMacro<'a, T, F1, F2> {
    pub fn new(
        trigger: impl AcceptsKeycode,
        on_toggle_on: F1,
        on_toggle_off: F2,
    ) -> StickyMacro<'a, T, F1, F2> {
        StickyMacro {
            keycode: trigger.to_u32(),
            on_toggle_on,
            on_toggle_off,
            active: 0,
            phantom: core::marker::PhantomData,
        }
    }
}

impl<T: USBKeyOut, F1: FnMut(&mut T), F2: FnMut(&mut T)> ProcessKeys<T>
    for StickyMacro<'_, T, F1, F2>
{
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
        for (event, status) in events.iter_mut() {
            //a sticky key
            // on press if not active -> active
            // on 2nd release if active -> deactivate
            match event {
                Event::KeyPress(kc) => {
                    if kc.keycode == self.keycode {
                        if self.active == 0 {
                            self.active = 1;
                            (self.on_toggle_on)(output);
                        } else {
                            self.active = 2;
                        }
                        *status = EventStatus::Handled;
                    }
                }
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.keycode {
                        if self.active == 2 {
                            (self.on_toggle_off)(output);
                        }
                        *status = EventStatus::Handled;
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
    }
}

/// A OneShot key
/// press it, on_toggle_on will be called,
/// on_toggle_off will be called after the next key
/// release of if the OneShot trigger is pressed again
pub struct OneShot<'a, T, F1: FnMut(&mut T), F2: FnMut(&mut T)> {
    keycode: u32,
    on_toggle_on: F1,
    on_toggle_off: F2,
    active: bool,
    phantom: core::marker::PhantomData<&'a T>,
}
impl<'a, T: USBKeyOut, F1: FnMut(&mut T), F2: FnMut(&mut T)> OneShot<'a, T, F1, F2> {
    pub fn new(
        trigger: impl AcceptsKeycode,
        on_toggle_on: F1,
        on_toggle_off: F2,
    ) -> OneShot<'a, T, F1, F2> {
        OneShot {
            keycode: trigger.to_u32(),
            on_toggle_on,
            on_toggle_off,
            active: false,
            phantom: core::marker::PhantomData,
        }
    }
}

impl<T: USBKeyOut, F1: FnMut(&mut T), F2: FnMut(&mut T)> ProcessKeys<T> for OneShot<'_, T, F1, F2> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
        for (event, status) in events.iter_mut() {
            //a sticky key
            // on press if not active -> active
            // on other key release -> deactivate
            // on press if active -> deactive
            // on release -> noop?
            match event {
                Event::KeyPress(kc) => {
                    if kc.keycode == self.keycode {
                        *status = EventStatus::Handled;
                        if self.active {
                            self.active = false;
                            (self.on_toggle_off)(output);
                        } else {
                            self.active = true;
                            (self.on_toggle_on)(output);
                        }
                    }
                }
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.keycode {
                        *status = EventStatus::Handled;
                    } else {
                        self.active = false;
                        (self.on_toggle_off)(output);
                    }
                }
                Event::TimeOut(_) => {}
            }
        }
    }
}

pub struct TapDance<'a, T, F> {
    trigger: u32,
    tap_count: u8,
    on_tap_complete: F, //todo: should we differentiate between timeout and other key by passing an enum?
    //todo: add on_each_tap...
    timeout_ms: u16,
    phantom: core::marker::PhantomData<&'a T>,
}

impl<'a, T: USBKeyOut, F: FnMut(u8, &mut T)> TapDance<'a, T, F> {
    pub fn new(trigger: impl AcceptsKeycode, on_tap_complete: F) -> TapDance<'a, T, F> {
        TapDance {
            trigger: trigger.to_u32(),
            tap_count: 0,
            on_tap_complete,
            timeout_ms: 250,
            phantom: core::marker::PhantomData,
        }
    }
}

impl<T: USBKeyOut, F: FnMut(u8, &mut T)> ProcessKeys<T> for TapDance<'_, T, F> {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
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
                            (self.on_tap_complete)(self.tap_count, output);
                            self.tap_count = 0;
                        }
                    } else {
                        self.tap_count += 1;
                        *status = EventStatus::Handled;
                    }
                }
                Event::TimeOut(ms_since_last) => {
                    if self.tap_count > 0 && *ms_since_last > self.timeout_ms {
                        (self.on_tap_complete)(self.tap_count, output);
                        self.tap_count = 0;
                    }
                }
            }
        }
    }
}

pub struct SpaceCadet<'a, T, F1, F2> {
    trigger: u32,
    on_activate: F1,
    on_deactivate: F2,
    press_number: u8,
    down: bool,
    activated: bool,
    phantom: core::marker::PhantomData<&'a T>,
}

impl<'a, T: USBKeyOut, F1: FnMut(&mut T), F2: FnMut(&mut T)> SpaceCadet<'a, T, F1, F2> {
    pub fn new(
        trigger: impl AcceptsKeycode,
        on_activate: F1,
        on_deactivate: F2,
    ) -> SpaceCadet<'a, T, F1, F2> {
        SpaceCadet {
            trigger: trigger.to_u32(),
            on_activate,
            on_deactivate,
            press_number: 0,
            down: false,
            activated: false,
            phantom: core::marker::PhantomData,
        }
    }
}

impl<T: USBKeyOut, F1: FnMut(&mut T), F2: FnMut(&mut T)> ProcessKeys<T>
    for SpaceCadet<'_, T, F1, F2>
{
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
        let mut initial_keypress_status: Option<EventStatus> = None;
        for (event, status) in iter_unhandled_mut(events) {
            match event {
                Event::KeyRelease(kc) => {
                    if kc.keycode == self.trigger {
                        self.down = false;
                        if kc.running_number == self.press_number + 1 {
                            // a tap
                            //let the downstream handle it!
                            initial_keypress_status = Some(EventStatus::Unhandled);
                        } else {
                            (self.on_deactivate)(output);
                            *status = EventStatus::Handled;
                            initial_keypress_status = Some(EventStatus::Handled);
                        }
                    }
                }
                Event::KeyPress(kc) => {
                    if kc.keycode == self.trigger {
                        *status = EventStatus::Ignored; //skip the scan this time
                        self.press_number = kc.running_number;
                        self.down = true
                    } else if self.down {
                        //trigger has been seen..
                        if !self.activated {
                            (self.on_activate)(output);
                        }
                        self.activated = true;
                        initial_keypress_status = Some(EventStatus::Ignored);
                        //remeber, this is a non-related keypress.
                        //*status = EventStatus::Handled;
                    }
                }
                Event::TimeOut(_) => {}
            }
        }

        match initial_keypress_status {
            Some(new_status) => {
                for (event, status) in events.iter_mut() {
                    match event {
                        Event::KeyPress(kc) => {
                            if kc.keycode == self.trigger {
                                *status = new_status;
                            }
                        }
                        _ => {}
                    }
                }
            }
            None => {}
        }
    }
}

pub struct AutoShift {
    shift_letters: bool,
    shift_numbers: bool,
    shift_special: bool,
    threshold_ms: u16,
}

impl AutoShift {
    pub fn new(threshold_ms: u16) -> AutoShift {
        AutoShift {
            shift_letters: true,
            shift_numbers: true,
            shift_special: true,
            threshold_ms,
        }
    }

    fn should_autoshift(&self, keycode: u32) -> bool {
        return (self.shift_letters
            && keycode >= KeyCode::A.to_u32()
            && keycode <= KeyCode::Z.to_u32())
            | (self.shift_numbers
                && keycode >= KeyCode::Kb1.to_u32()
                && keycode <= KeyCode::Kb0.to_u32())
            | (self.shift_special
                && keycode >= KeyCode::Minus.to_u32()
                && keycode <= KeyCode::Slash.to_u32());
    }
}

impl<T: USBKeyOut> ProcessKeys<T> for AutoShift {
    fn process_keys(&mut self, events: &mut Vec<(Event, EventStatus)>, output: &mut T) -> () {
        let mut presses = Vec::new();
        let mut handled = Vec::new();
        for (event, status) in iter_unhandled_mut(events) {
            match event {
                Event::KeyPress(kc) => {
                    if self.should_autoshift(kc.keycode) {
                        *status = EventStatus::Ignored;
                        presses.push((kc.keycode, kc.ms_since_last));
                    }
                }
                Event::KeyRelease(kc) => {
                    if self.should_autoshift(kc.keycode) {
                        for (other_keycode, timestamp) in presses.iter() {
                            if *other_keycode == kc.keycode {
                                let delta = kc.ms_since_last - timestamp;
                                if delta >= self.threshold_ms {
                                    output.send_keys(&[
                                        KeyCode::LShift,
                                        (kc.keycode as u8).try_into().unwrap(),
                                    ])
                                } else {
                                    output.send_keys(&[(kc.keycode as u8).try_into().unwrap()])
                                }
                                handled.push(kc.keycode)
                            }
                        }
                        *status = EventStatus::Handled;
                    }
                }
                _ => {}
            }
        }
        if !handled.is_empty() {
            for (event, status) in events.iter_mut() {
                match event {
                    Event::KeyPress(kc) => {
                        if handled.contains(&kc.keycode) {
                            *status = EventStatus::Handled;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
//#[macro_use]
//extern crate std;
mod tests {
    #[allow(unused_imports)]
    use crate::key_codes::KeyCode;
    #[allow(unused_imports)]
    use crate::test_helpers::{check_output, KeyOutCatcher};

    use crate::handlers::{
        AutoShift, Layer, LayerAction, Leader, MatchResult, OneShot, PressReleaseMacro, SpaceCadet,
        StickyMacro, TapDance, USBKeyboard, UnicodeKeyboard,
    };
    #[allow(unused_imports)]
    use crate::{Keyboard, KeyboardState, USBKeyOut, UnicodeSendMode, UNICODE_BELOW_256};
    #[allow(unused_imports)]
    use no_std_compat::prelude::v1::*;

    #[allow(unused_imports)]
    use no_std_compat::cell::RefCell;
    #[allow(unused_imports)]
    use no_std_compat::rc::Rc;

    #[test]
    fn test_usbkeyboard_single_key() {
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::A]]);
        assert!(!keyboard.events.is_empty());
        keyboard.add_keyrelease(KeyCode::A, 20);
        assert!(keyboard.events.len() == 2);
        keyboard.output.clear();
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        assert!(keyboard.events.is_empty());
    }
    #[test]
    fn test_usbkeyboard_multiple_key() {
        use crate::key_codes::KeyCode::*;
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        keyboard.add_keypress(A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[A]]);
        assert!(!keyboard.events.is_empty());

        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[A, X]]);
        assert!(!keyboard.events.is_empty());

        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::A, 20);
        assert!(keyboard.events.len() == 3);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[X]]);
        assert!(!keyboard.events.is_empty());

        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::X, 20);
        assert!(keyboard.events.len() == 2);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        assert!(keyboard.events.is_empty());
    }

    #[test]
    fn test_unicode_keyboard_linux() {
        use crate::key_codes::KeyCode::*;
        let ub = UnicodeKeyboard {};
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(ub));
        keyboard.output.state().unicode_mode = UnicodeSendMode::Linux;
        //no output on press
        keyboard.add_keypress(0x00E4u32 + UNICODE_BELOW_256, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.reports.len() == 0);
        assert!(keyboard.events.is_empty()); // we eat the keypress though

        keyboard.add_keyrelease(0x00E4 + UNICODE_BELOW_256, 0);
        keyboard.handle_keys().unwrap();
        check_output(
            &keyboard,
            &[
                &[U, LShift, LCtrl],
                &[E, LShift, LCtrl],
                &[Kb4, LShift, LCtrl],
                &[],
            ],
        );

        assert!(keyboard.events.is_empty()); // we eat the keypress though
    }

    #[test]
    fn test_unicode_keyboard_wincompose() {
        use crate::key_codes::KeyCode::*;
        let ub = UnicodeKeyboard {};
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(ub));
        keyboard.output.state().unicode_mode = UnicodeSendMode::WinCompose;
        //no output on press
        keyboard.add_keypress(0x03B4u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.output.reports.len() == 0);
        assert!(keyboard.events.is_empty()); // we eat the keypress though

        keyboard.add_keyrelease(0x03B4, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[RAlt], &[U], &[Kb3], &[B], &[Kb4], &[]]);
        assert!(keyboard.events.is_empty()); // we eat the keypress though
    }

    #[test]
    fn test_unicode_while_depressed() {
        use crate::key_codes::KeyCode::*;
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(UnicodeKeyboard {}));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        keyboard.output.state().unicode_mode = UnicodeSendMode::WinCompose;
        keyboard.add_keypress(A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[A]]);
        keyboard.output.clear();
        keyboard.add_keypress(0x3B4u32, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[A]]);
        keyboard.add_keyrelease(0x3B4, 0);
        keyboard.output.clear();
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[RAlt], &[U], &[Kb3], &[B], &[Kb4], &[], &[A]]);
        keyboard.add_keyrelease(A, 0);
        keyboard.output.clear();
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        assert!(keyboard.events.is_empty());
    }

    #[test]
    fn test_panic_on_unhandled() {
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        keyboard.add_keypress(0xF0000u32, 0);
        assert!(keyboard.handle_keys().is_err());
    }

    #[test]
    fn test_toggle_macro() {
        let down_counter = RefCell::new(0);
        let up_counter = RefCell::new(0);
        let t = OneShot::new(
            0xF0000u32,
            |output: &mut KeyOutCatcher| {
                output.send_keys(&[KeyCode::H]);
                let mut dc = down_counter.borrow_mut();
                *dc += 1;
            },
            |_output| {
                let mut dc = up_counter.borrow_mut();
                *dc += 1;
            },
        );

        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(t));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        //first press - sets
        keyboard.add_keypress(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 0);
        assert!(keyboard.events.is_empty());
        check_output(&keyboard, &[&[KeyCode::H], &[]]);
        keyboard.output.clear();

        //first release - no change
        keyboard.add_keyrelease(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 0);
        assert!(keyboard.events.is_empty());

        //second press - unsets
        keyboard.add_keypress(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 1);
        assert!(keyboard.events.is_empty());

        //second release - no change
        keyboard.add_keyrelease(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 1);
        assert!(keyboard.events.is_empty());

        //third press - sets
        keyboard.add_keypress(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 2);
        assert!(*up_counter.borrow() == 1);
        assert!(keyboard.events.is_empty());

        keyboard.add_keypress(KeyCode::A, 20);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 2);
        assert!(*up_counter.borrow() == 1);

        keyboard.add_keyrelease(KeyCode::A, 20);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 2);
        assert!(*up_counter.borrow() == 2);

        //third release - no change
        keyboard.add_keyrelease(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 2);
        assert!(*up_counter.borrow() == 2);
        assert!(keyboard.events.is_empty());

        //fourth press - sets
        keyboard.add_keypress(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 3);
        assert!(*up_counter.borrow() == 2);
        assert!(keyboard.events.is_empty());
    }

    #[test]
    fn test_press_release() {
        let down_counter = RefCell::new(0);
        let up_counter = RefCell::new(0);
        let t = PressReleaseMacro::new(
            0xF0000u32,
            Option::Some(|output: &mut KeyOutCatcher| {
                //todo: why do we need to define the type here?
                output.send_keys(&[KeyCode::H]);
                let mut dc = down_counter.borrow_mut();
                *dc += 1;
            }),
            Option::Some(|output: &mut KeyOutCatcher| {
                let mut dc = up_counter.borrow_mut();
                *dc += 1;
                output.send_keys(&[KeyCode::I]);
            }),
        );

        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(t));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        //first press - sets
        keyboard.add_keypress(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 0);
        assert!(keyboard.events.is_empty());
        check_output(&keyboard, &[&[KeyCode::H], &[]]);
        keyboard.output.clear();

        //first release - no change
        keyboard.add_keyrelease(0xF0000u32, 0);
        keyboard.handle_keys().unwrap();
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 1);
        check_output(&keyboard, &[&[KeyCode::I], &[]]);
        keyboard.output.clear();
    }
    #[test]
    fn test_layer_rewrite() {
        let l = Layer::new(vec![(
            KeyCode::A,
            LayerAction::RewriteTo(KeyCode::X.into()),
        )]);

        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let layer_id = keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        keyboard.enable_handler(layer_id);
        keyboard.add_keypress(KeyCode::B, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::B, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();

        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();

        check_output(
            &keyboard,
            &[&[KeyCode::B], &[], &[KeyCode::X], &[], &[KeyCode::X], &[]],
        );

        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keypress(KeyCode::B, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::B, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(
            &keyboard,
            &[&[KeyCode::X], &[KeyCode::X, KeyCode::B], &[KeyCode::X], &[]],
        );

        keyboard.output.clear();
        keyboard.disable_handler(layer_id);
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::A], &[]]);

        keyboard.output.clear();
        keyboard.enable_handler(layer_id);
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::X], &[]]);

        //TODO: what happens when you disable the layer in the middle?
        // I suspect that we will keep repeating one of the keycodes.
        // what would be the sensible thing to happen? How can we achive this?
        // possibly by clearing the keyboard events whenever a layer toggle happens?
    }
    #[test]
    fn test_layer_rewrite_shifted() {
        let l = Layer::new(vec![(
            KeyCode::A,
            LayerAction::RewriteToShifted(KeyCode::M.into(), KeyCode::Z.into()),
        )]);

        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let layer_id = keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        keyboard.enable_handler(layer_id);
        assert!(!keyboard.output.state().shift);
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::M], &[]]);
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::LShift, 0);
        keyboard.handle_keys().unwrap();
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(
            &keyboard,
            &[&[KeyCode::LShift], &[KeyCode::LShift, KeyCode::Z]],
        );
        assert!(keyboard.output.state().shift);
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift]]);
        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::LShift, 0);
        keyboard.handle_keys().unwrap();
        dbg!(keyboard.output.state());
        assert!(!(keyboard.output.state().shift));
        check_output(&keyboard, &[&[]]);
    }

    #[test]
    fn test_tapdance() {
        let l = TapDance::new(
            KeyCode::X,
            |tap_count, output: &mut KeyOutCatcher| match tap_count {
                1 => output.send_keys(&[KeyCode::A]),
                2 => output.send_keys(&[KeyCode::B]),
                3 => output.send_keys(&[KeyCode::C]),
                _ => output.send_keys(&[KeyCode::D]),
            },
        );
        let timeout = l.timeout_ms;

        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        //simplest case, one press/release then another key
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 10);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::Z, 20);
        keyboard.handle_keys().unwrap();
        //       keyboard.add_keyrelease(KeyCode::Z, 30);
        check_output(&keyboard, &[&[KeyCode::A], &[KeyCode::Z]]);
        keyboard.add_keyrelease(KeyCode::Z, 20);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.events.is_empty());

        //two taps, then another key
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 10);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::X, 20);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 30);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::Z, 40);
        keyboard.handle_keys().unwrap();
        //        keyboard.add_keyrelease(KeyCode::Z, 50);
        check_output(&keyboard, &[&[KeyCode::B], &[KeyCode::Z]]);
        keyboard.add_keyrelease(KeyCode::Z, 20);
        keyboard.handle_keys().unwrap();
        assert!(keyboard.events.is_empty());

        //three taps, then a time out
        keyboard.output.clear();
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 10);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::X, 20);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 30);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::X, 20);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 30);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_timeout(timeout - 1);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_timeout(timeout + 1);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::C], &[]]);
    }

    #[test]
    fn test_sticky_macro() {
        let l = StickyMacro::new(
            KeyCode::X,
            |output: &mut KeyOutCatcher| output.send_keys(&[KeyCode::A]),
            |output: &mut KeyOutCatcher| output.send_keys(&[KeyCode::B]),
        );

        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        //activate
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::A], &[]]);
        keyboard.output.clear();

        //ignore
        keyboard.add_keyrelease(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        //ignore
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        //deactivate
        keyboard.add_keyrelease(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::B], &[]]);
    }

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

    #[test]
    fn test_spacecadet() {
        let down_counter = RefCell::new(0);
        let up_counter = RefCell::new(0);
        let l = SpaceCadet::new(
            KeyCode::X,
            |output: &mut KeyOutCatcher| {
                println!("activate");
                let mut c = down_counter.borrow_mut();
                *c += 1;
                output.send_keys(&[KeyCode::A])
            },
            |output: &mut KeyOutCatcher| {
                println!("deactivate");
                let mut c = up_counter.borrow_mut();
                *c += 1;
                output.send_keys(&[KeyCode::B])
            },
        );

        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        //the tap...
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 10);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::X]]);
        keyboard.output.clear();

        assert!(keyboard.events.is_empty());

        //the modifier
        println!("as mod");
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::Z, 0);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[KeyCode::A], &[KeyCode::Z]]);
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 0);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::Z, 0);
        keyboard.handle_keys().unwrap();
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, 10);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::B], &[]]);
        assert!(*down_counter.borrow() == 1);
        assert!(*up_counter.borrow() == 1);
        keyboard.output.clear();
    }

    #[test]
    fn test_autoshift() {
        let threshold = 200;
        let l = AutoShift::new(threshold);

        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, threshold - 1);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::X], &[]]);
        keyboard.output.clear();
        dbg!(&keyboard.events);
        assert!(keyboard.events.is_empty());

        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::X, threshold + 1);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[KeyCode::X, KeyCode::LShift], &[]]);
        keyboard.output.clear();
    }
    #[test]
    fn test_autoshift_no_letters() {
        let threshold = 200;
        let mut l = AutoShift::new(threshold);
        l.shift_letters = false;

        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(l));
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        keyboard.add_keypress(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::Kb1, threshold - 1);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::Kb1], &[]]);
        keyboard.output.clear();
        dbg!(&keyboard.events);
        assert!(keyboard.events.is_empty());

        keyboard.add_keypress(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::Kb1, threshold + 1);
        keyboard.handle_keys().unwrap();
        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[KeyCode::Kb1, KeyCode::LShift], &[]]);
        keyboard.output.clear();

        //this now get's handled by the usb keyboard.
        keyboard.add_keypress(KeyCode::X, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::X]]);
        keyboard.output.clear()
    }

    #[test]
    fn test_modifiers_add_left_keycodes() {
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        keyboard.add_keypress(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::Kb1]]);

        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        keyboard.output.clear();

        keyboard.output.state().shift = true;

        keyboard.add_keypress(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::Kb1, KeyCode::LShift]]);

        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift]]);
        keyboard.output.clear();

        keyboard.output.state().shift = false;
        keyboard.output.state().ctrl = true;

        keyboard.add_keypress(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::Kb1, KeyCode::LCtrl]]);

        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LCtrl]]);
        keyboard.output.clear();

        keyboard.output.state().ctrl = false;
        keyboard.output.state().alt = true;

        keyboard.add_keypress(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::Kb1, KeyCode::LAlt]]);

        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LAlt]]);
        keyboard.output.clear();

        keyboard.output.state().alt = false;
        keyboard.output.state().meta = true;

        keyboard.add_keypress(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::Kb1, KeyCode::LGui]]);

        keyboard.output.clear();
        keyboard.add_keyrelease(KeyCode::Kb1, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LGui]]);
    }
    #[test]
    fn test_modifiers_set_by_keycodes() {
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        keyboard.add_keypress(KeyCode::LShift, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift]]);
        assert!(keyboard.output.state().shift);
        assert!(!keyboard.output.state().alt);
        assert!(!keyboard.output.state().ctrl);
        assert!(!keyboard.output.state().meta);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::LAlt, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift, KeyCode::LAlt]]);
        assert!(keyboard.output.state().shift);
        assert!(keyboard.output.state().alt);
        assert!(!keyboard.output.state().ctrl);
        assert!(!keyboard.output.state().meta);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::LCtrl, 0);
        keyboard.handle_keys().unwrap();
        check_output(
            &keyboard,
            &[&[KeyCode::LShift, KeyCode::LAlt, KeyCode::LCtrl]],
        );
        assert!(keyboard.output.state().shift);
        assert!(keyboard.output.state().alt);
        assert!(keyboard.output.state().ctrl);
        assert!(!keyboard.output.state().meta);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::LGui, 0);
        keyboard.handle_keys().unwrap();
        check_output(
            &keyboard,
            &[&[
                KeyCode::LShift,
                KeyCode::LAlt,
                KeyCode::LCtrl,
                KeyCode::LGui,
            ]],
        );
        assert!(keyboard.output.state().shift);
        assert!(keyboard.output.state().alt);
        assert!(keyboard.output.state().ctrl);
        assert!(keyboard.output.state().meta);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::LGui, 0);
        keyboard.handle_keys().unwrap();

        check_output(
            &keyboard,
            &[&[KeyCode::LShift, KeyCode::LAlt, KeyCode::LCtrl]],
        );
        assert!(keyboard.output.state().shift);
        assert!(keyboard.output.state().alt);
        assert!(keyboard.output.state().ctrl);
        assert!(!keyboard.output.state().meta);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::LCtrl, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift, KeyCode::LAlt]]);
        assert!(keyboard.output.state().shift);
        assert!(keyboard.output.state().alt);
        assert!(!keyboard.output.state().ctrl);
        assert!(!keyboard.output.state().meta);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::LAlt, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift]]);
        assert!(keyboard.output.state().shift);
        assert!(!keyboard.output.state().alt);
        assert!(!keyboard.output.state().ctrl);
        assert!(!keyboard.output.state().meta);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::LShift, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        assert!(!keyboard.output.state().shift);
        assert!(!keyboard.output.state().alt);
        assert!(!keyboard.output.state().ctrl);
        assert!(!keyboard.output.state().meta);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::RShift, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::RShift]]);
        assert!(keyboard.output.state().shift);
        assert!(!keyboard.output.state().alt);
        assert!(!keyboard.output.state().ctrl);
        assert!(!keyboard.output.state().meta);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::RAlt, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::RShift, KeyCode::RAlt]]);
        assert!(keyboard.output.state().shift);
        assert!(keyboard.output.state().alt);
        assert!(!keyboard.output.state().ctrl);
        assert!(!keyboard.output.state().meta);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::RCtrl, 0);
        keyboard.handle_keys().unwrap();
        check_output(
            &keyboard,
            &[&[KeyCode::RShift, KeyCode::RAlt, KeyCode::RCtrl]],
        );
        assert!(keyboard.output.state().shift);
        assert!(keyboard.output.state().alt);
        assert!(keyboard.output.state().ctrl);
        assert!(!keyboard.output.state().meta);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::RGui, 0);
        keyboard.handle_keys().unwrap();
        check_output(
            &keyboard,
            &[&[
                KeyCode::RShift,
                KeyCode::RAlt,
                KeyCode::RCtrl,
                KeyCode::RGui,
            ]],
        );
        assert!(keyboard.output.state().shift);
        assert!(keyboard.output.state().alt);
        assert!(keyboard.output.state().ctrl);
        assert!(keyboard.output.state().meta);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::RGui, 0);
        keyboard.handle_keys().unwrap();
        check_output(
            &keyboard,
            &[&[KeyCode::RShift, KeyCode::RAlt, KeyCode::RCtrl]],
        );
        assert!(keyboard.output.state().shift);
        assert!(keyboard.output.state().alt);
        assert!(keyboard.output.state().ctrl);
        assert!(!keyboard.output.state().meta);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::RCtrl, 0);
        keyboard.handle_keys().unwrap();

        dbg!(&keyboard.output.reports);
        check_output(&keyboard, &[&[KeyCode::RShift, KeyCode::RAlt]]);
        assert!(keyboard.output.state().shift);
        assert!(keyboard.output.state().alt);
        assert!(!keyboard.output.state().ctrl);
        assert!(!keyboard.output.state().meta);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::RAlt, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::RShift]]);
        assert!(keyboard.output.state().shift);
        assert!(!keyboard.output.state().alt);
        assert!(!keyboard.output.state().ctrl);
        assert!(!keyboard.output.state().meta);
        keyboard.output.clear();

        keyboard.add_keyrelease(KeyCode::RShift, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[]]);
        assert!(!keyboard.output.state().shift);
        assert!(!keyboard.output.state().alt);
        assert!(!keyboard.output.state().ctrl);
        assert!(!keyboard.output.state().meta);
        keyboard.output.clear();

        keyboard.add_keypress(KeyCode::LShift, 0);
        keyboard.add_keypress(KeyCode::RShift, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::LShift, KeyCode::RShift]]);
        assert!(keyboard.output.state().shift);
        assert!(!keyboard.output.state().alt);
        assert!(!keyboard.output.state().ctrl);
        assert!(!keyboard.output.state().meta);
        keyboard.output.clear();
    }

    #[test]
    fn test_enable_disable() {
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        let prm = PressReleaseMacro::new(
            KeyCode::A,
            Some(|output: &mut KeyOutCatcher| {
                output.send_keys(&[KeyCode::B]);
            }),
            Some(|output: &mut KeyOutCatcher| {
                output.send_keys(&[KeyCode::C]);
            }),
        );
        let no = keyboard.add_handler(Box::new(prm));
        keyboard.add_handler(Box::new(USBKeyboard::new()));

        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::B], &[]]);
        keyboard.add_keyrelease(KeyCode::A, 10);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::B], &[], &[KeyCode::C], &[]]);

        keyboard.output.clear();
        keyboard.disable_handler(no);
        keyboard.add_keypress(KeyCode::A, 0);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::A]]);
        keyboard.add_keyrelease(KeyCode::A, 10);
        keyboard.handle_keys().unwrap();
        check_output(&keyboard, &[&[KeyCode::A], &[]]);
    }

}
