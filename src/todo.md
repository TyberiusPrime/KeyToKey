Open tasks:

 * include physical key id in Key and use it to discover matching
   pairs - necessary to correctly handle layer on/off while a key is depressed
 * TapAndLongTap functionality
 * debug spacecadet
 * rework leader
 * debug&rework tapdance
 * sticky macro to trait interface
 * improve readme


Sort: 
// shift remaps on layers (ie. disassociate the premade shift-combos) testing
// oneshot deactivate if released after x seconds
// leader does not work
// space cadet does not work
// combos
// tapdance enhancemeants, on_each_tap, and max_taps?
// toggle on x presses? - should be a tapdance impl?
// premade toggle/oneshot modifiers
// key lock (repeat next key until it is pressed again)
// mouse keys? - probably out of scope of this libary
// steganograpyh
// unsupported: disabling a layer when one of it's rewriteTo are active?

Done

 * refactor modifiers to be kept in the enable bit (saves some code in variants to Action)
 * use smallbitvec for the enablers