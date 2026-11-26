use rs_js::minify;

mod support;

use support::assert_golden;

#[test]
fn minify_embeddable_matches_the_production_golden() {
    let source = r#"
export default function boot() {
  const label = "hello";
  return { label, value: 42 };
}
window.runtime = boot();
"#;
    let output = minify("runtime.mjs", source)
        .module()
        .embeddable()
        .run()
        .expect("embeddable minify works");
    let surface = format!(
        "flavor={}\nmangled={}\ninput_bytes={}\noutput_bytes={}\ncode={}\n",
        output.source_flavor.as_str(),
        output.mangling_applied,
        output.input_bytes,
        output.output_bytes,
        output.code,
    );
    assert_golden("embeddable_minify_surface.txt", &surface);
}

#[test]
fn obfuscate_source_matches_the_production_golden() {
    let source = r#"
function boot(answer) {
  const message = "hello";
  return { message, answer };
}
window.runtime = boot(42);
"#;
    let output = minify("runtime.js", source)
        .javascript()
        .compatible_csp()
        .obfuscate()
        .run()
        .expect("obfuscate should succeed");
    let surface = format!(
        "flavor={}\nmangled={}\noutput_bytes={}\ncode={}\n",
        output.source_flavor.as_str(),
        output.mangling_applied,
        output.output_bytes,
        output.code,
    );
    assert_golden("obfuscate_surface.txt", &surface);
}

#[test]
fn obfuscate_source_skips_mangling_for_indirect_eval_patterns() {
    let source = "globalThis.eval('40 + 2'); const value = 42; window.answer = value;";
    let output = minify("runtime.js", source)
        .javascript()
        .compatible_csp()
        .obfuscate()
        .run()
        .expect("compatible obfuscation should still work");

    assert!(!output.mangling_applied);
    assert!(output.report.indirect_eval_calls >= 1);
}
