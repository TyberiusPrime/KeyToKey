use no_std_compat::prelude::v1::*;

#[derive(PartialEq, Debug)]
pub struct Key {
    pub keycode: u32,
    pub ms_since_last: u16,
    pub running_number: u8,
    pub flag: u8, //Todo: express this better
                  //bit 0 is used by Usbkeyboard to decide whether a KeyPress has ever been sent
                  //(or kept back by a different handler so far)
                  //bit1 is used to protect against double rewrites
}

impl Key {
    pub fn new(keycode: u32) -> Key {
        Key {
            keycode,
            ms_since_last: 0,
            running_number: 0,
            flag: 0,
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum Event {
    KeyPress(Key),
    KeyRelease(Key),
    TimeOut(u16),
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum EventStatus {
    Unhandled,
    Handled,
    Ignored,
}

pub fn iter_unhandled_mut(
    events: &mut Vec<(Event, EventStatus)>,
) -> impl DoubleEndedIterator<Item = &mut (Event, EventStatus)> {
    events
        .iter_mut()
        .filter(|(_e, status)| EventStatus::Unhandled == *status)
}
