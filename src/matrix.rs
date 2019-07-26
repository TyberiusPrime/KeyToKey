//use no_std_compat::prelude::v1::*;
use smallbitvec::{SmallBitVec, sbvec};
use crate::{Keyboard, USBKeyOut};


struct MatrixToStream<'a>{
    last_state: SmallBitVec,
    translation: &'a [u32],
}

impl MatrixToStream<'_> {
    fn new<'a> (no_of_keys: u8,
        translation: &'a [u32]) -> MatrixToStream<'a> {
            MatrixToStream {
            last_state: sbvec![false; no_of_keys as usize],
            translation,
        }
    }

    fn update<T: USBKeyOut>(&mut self, new_state: &SmallBitVec, keyboard: &mut Keyboard<T>, ms_since_last: u16) {
        assert!(new_state.len() == self.last_state.len());
        for (ii, (old, new)) in self.last_state.iter().zip(new_state).enumerate() {
            if old != new {
                match new {
                    true => keyboard.add_keypress(self.translation[ii], ms_since_last),
                    false => keyboard.add_keyrelease(self.translation[ii], ms_since_last),
                };
            }
        }
        for ii in 0..self.last_state.len() {
            self.last_state.set(ii, new_state.get(ii).unwrap());
        }
    }
}