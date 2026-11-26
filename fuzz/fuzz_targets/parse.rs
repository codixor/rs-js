#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &str| {
    let _ = rs_js::minify("input.js", input)
        .javascript()
        .analyze()
        .report();
});
