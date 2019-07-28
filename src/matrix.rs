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
        let mut any_changed = false;
        for (ii, (old, new)) in self.last_state.iter().zip(new_state).enumerate() {
            if old != new {
                match new {
                    true => keyboard.add_keypress(self.translation[ii], ms_since_last),
                    false => keyboard.add_keyrelease(self.translation[ii], ms_since_last),
                };
                keyboard.handle_keys().ok();
                keyboard.clear_unhandled();
                any_changed = true;
            }
        }
        if !any_changed {
            keyboard.add_timeout(ms_since_last);
                keyboard.handle_keys().ok();
                keyboard.clear_unhandled();
        }
        for ii in 0..self.last_state.len() {
            self.last_state.set(ii, new_state.get(ii).unwrap());
        }
    }
}



#[cfg(test)]

mod tests {
use no_std_compat::prelude::v1::*;
        use crate::{Keyboard, USBKeyboard};
        use crate::test_helpers::{KeyOutCatcher, check_output, TimeoutLogger};
        use crate::key_codes::KeyCode;
        use crate::matrix::MatrixToStream;
        use crate::AcceptsKeycode;
     use smallbitvec::{sbvec};
    #[test]
    fn test_matrix_to_stream() {
        let trans = [KeyCode::A.to_u32(), KeyCode::Z.to_u32()];
       let mut matrix = MatrixToStream::new(2, &trans);
        let mut state = sbvec![false; 2];
        let mut keyboard = Keyboard::new(KeyOutCatcher::new());
        keyboard.add_handler(Box::new(USBKeyboard::new()));
        keyboard.add_handler(Box::new(TimeoutLogger::new(KeyCode::X, 100)));
        state.set(0,true);
        matrix.update(&state, &mut keyboard, 120);
        check_output(&keyboard, &[&[KeyCode::A]]);
        matrix.update(&state, &mut keyboard, 240);
        check_output(&keyboard, &[&[KeyCode::A], &[KeyCode::A], &[KeyCode::X]]);
        state.set(0,false);
        matrix.update(&state, &mut keyboard, 240);
        check_output(&keyboard, &[&[KeyCode::A], &[KeyCode::A], &[KeyCode::X], &[]]);
        matrix.update(&state, &mut keyboard, 240);
        check_output(&keyboard, &[&[KeyCode::A], &[KeyCode::A], &[KeyCode::X], &[], &[], &[KeyCode::X]]);
        matrix.update(&state, &mut keyboard, 50);
        check_output(&keyboard, &[&[KeyCode::A], &[KeyCode::A], &[KeyCode::X], &[], &[], &[KeyCode::X], &[]]);


    }
}