Open tasks:

 * include physical key id in Key and use it to discover matching
   pairs - necessary to correctly handle layer on/off while a key is depressed
 * TapAndLongTap functionality
 * debug spacecadet
 * rework leader
 * debug&rework tapdance
 * improve readme


Sort: 
// shift remaps on layers (ie. disassociate the premade shift-combos) testing
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