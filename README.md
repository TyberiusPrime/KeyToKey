# KeyToKey

KeyToKey is a Rust libary for building keyboard firmwares.

Basically, a keyboard firmware 
  a) reads key-presses, 
  b) translates them
  c) and outputs the results to a computer.

KeyToKey's role is in b - it takes a stream of events,
such as key presses, releases and timeouts, and translates
them using a series of handler trait objects into reports
that can easily be converted to the format USB expects.

This is inspired by the [QMK](https://github.com/qmk/qmk_firmware)
mechanical keyboard firmware, which probably is the most feature complete
keyboard firmware to date. Alas it's C, and an amazing ball of ifdefs that
usually needs more flash than the micros it's targeting offer.

The trait object oriented approach chosen here allows composition
of arbitrarily complex keyboard layouts - and unit testing them without hardware.

To get started, maybe check out the [reference firmware](https://github.com/TyberiusPrime/stm32f103_k2k).

KeyToKey operates on key codes that are u32 representing unicode key points.
(The USB keycodes are nested in the first 'private area' of the unicode code set,
and the rest of the private area is free to be used by the keyboard implementor.)

A basic keyboard uses two handlers - a handlers::UnicodeKeyboard, which sends
the OS-specific magic 'enter-an-arbitrary-unicode-keypoint' for any key code outside
of the private areas, and a handlers::USBKeyboard which handels all the usual
'tell the computer which buttons are pressed' functionality including modifiers. 
USBKeyboard does not limit the number of simultanious keys, but the downstream translation into USB might restrict
to the usual 6 key rollover.


Basic features
 * works as a regular USB keyboard
 * arbirtrary unicode input in linux and windows

Advanced Features working
 * Layers (which can rewrite key codes, conditionally rewrite them based on shift status or send arbitrary strings, again dependeant on shift)
 * RewriteLayers (which can only rewrite key codes, but are much more memory efficient)
 * PressReleaseMacros (callbacks on key press / key release)
 * StickyMacros (Tap once to activate, again to deactivate){
 * OneShots (Press -> activate, deactivates after next any-not-one-shot key press - useful for modifiers or temporarily activated layers)
 * SpaceCadet (Do one thing on press-and-hold, a different thing on tap. For example a shift key that also outputs a '('))
 * Sequences (which don't intercept the keycodes, but then send a set of backspace presses, and then your action)

 Advanced features planned
  * Leader sequences (e.g. hit `leader` h e a r t to enter a heart emoji, or an arbitrary string)
  * TapDance (Count the number of taps on a key, pass the final count to a callback)
  * AutoShift - Short tap: lower case, longer tap: uppercase. Removes key repeat though.



Other rust firmwares / keyboard libraries

* https://gitlab.com/polymer-kb/firmware/polymer