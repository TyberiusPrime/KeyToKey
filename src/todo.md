Open tasks:

 * rework leader
 * debug&rework tapdance
 * improve readme
 * Layers: Allow passing triggers/action - Unfortunatly, you can't just use Trigger,
   since the impl USBKeyOut means it's generic, and turning
   it into a Trigger<T: USBKeyOut>  bubbles the output type into the LayerAction, then into the Layer, and then we run into some Sync/Send issue that I can't fathom
 * Consider passing absolute times so that LongTap can work no matter what other keys are pressed inbetween
 * escapeOff to more general premade turn-the-layers off thing.
 * test and document space cadet only working by itself - not when any other key is pressed
 * figure out how to send long strings - currently they readily explode the available ram,
   since each record is 8 bytes, and each character creates 4-12 records, so 72 characters is 2304..6912 bytes!

 * space cadet + dvorak apperantly blocks key repeat for non-trigger key?


Sort: 
// combos
// tapdance enhancemeants, on_each_tap, and max_taps?
// toggle on x presses? - should be a tapdance impl?
// key lock (repeat next key until it is pressed again)
// mouse keys? - probably out of scope of this libary
// steganograpyh
// unsupported: disabling a layer when one of it's rewriteTo are active?

Done

 * oneshot deactivate if released after x ms
 * oneshot deactivate if not consumed after x ms
 * refactor modifiers to be kept in the enable bit (saves some code in variants to Action)
 * use smallbitvec for the enablers
 * sticky macro to trait interface
 * TapAndLongTap functionality
 * correctly handle layer on/off while a key is depressed
 * debug spacecadet (actually a new implementation, but it seems to work ok)
 * interaction of space cadet and one shot - oneshot degenrates into modifier? - oneshot must come first
 * dvorak - replace with const something to safe on RAM
 * layer SendStringShifted test