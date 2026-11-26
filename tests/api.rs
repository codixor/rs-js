use rs_js::{SourceFlavor, compat, minify};

#[test]
fn analyze_javascript_exposes_report_for_simple_input() {
    let report = minify("input.js", "const answer = () => 40 + 2;")
        .javascript()
        .compatible_csp()
        .analyze()
        .report()
        .expect("analyze works");

    assert!(report.bindings >= 1);
    assert_eq!(report.direct_eval_calls, 0);
}

#[test]
fn obfuscate_javascript_runs_through_the_simple_public_api() {
    let source = "const sum = (a, b) => a + b; window.value = sum(1, 2);";
    let output = minify("runtime.js", source)
        .javascript()
        .compatible_csp()
        .obfuscate()
        .run()
        .expect("obfuscate works");

    assert_ne!(output.code, "");
    assert!(output.code.len() <= source.len());
    assert_eq!(output.source_flavor, SourceFlavor::Js);
    assert_eq!(output.input_bytes, source.len());
    assert_eq!(output.output_bytes, output.code.len());
}

#[test]
fn minify_embeddable_javascript_runs_through_the_simple_public_api() {
    let source = "export default function boot(){ return 42; } window.runtime = boot();";
    let output = minify("runtime.mjs", source)
        .module()
        .embeddable()
        .run()
        .expect("embeddable minify works");

    assert!(!output.code.contains("export default"));
    assert_ne!(output.code, "");
    assert_eq!(output.source_flavor, SourceFlavor::Mjs);
    assert_eq!(output.output_bytes, output.code.len());
}

#[test]
fn minify_builder_is_the_primary_fluent_api() {
    let output = minify("runtime.js", "const value = 40 + 2; window.answer = value;")
        .javascript()
        .strict_csp()
        .run()
        .expect("builder minify should work");

    assert_eq!(output.source_flavor, SourceFlavor::Js);
    assert!(output.output_bytes < output.input_bytes);
}

#[test]
fn compatibility_helpers_stay_available_under_compat() {
    let report =
        compat::analyze_source("input.js", "const answer = () => 40 + 2;").expect("compat works");

    assert!(report.bindings >= 1);
}
