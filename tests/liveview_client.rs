use std::{fs, path::PathBuf};

use rs_js::minify;

mod support;

use support::assert_golden;

#[test]
fn liveview_client_asset_stays_strict_csp_safe_and_minifiable() {
    let source = read_asset("liveview_client.js");
    let output = minify("liveview_client.js", &source)
        .javascript()
        .run()
        .expect("liveview client should minify under strict CSP");

    assert!(!output.report.has_strict_csp_violations());
    assert!(output.output_bytes < output.input_bytes);
    assert!(!output.code.contains("eval("));
    assert!(!output.code.contains("Function("));

    let surface = format!(
        "flavor={}\ninput_bytes={}\noutput_bytes={}\nmangled={}\nwarning_count={}\nstrict_csp_safe={}\noutput_hash={:016x}\n",
        output.source_flavor.as_str(),
        output.input_bytes,
        output.output_bytes,
        output.mangling_applied,
        output.report.warning_messages().len(),
        !output.report.has_strict_csp_violations(),
        stable_hash(&output.code),
    );
    assert_golden("liveview_client_surface.txt", &surface);
}

fn read_asset(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing asset {}", path.display()))
}

#[must_use]
fn stable_hash(input: &str) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;

    let mut hash = OFFSET;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}
