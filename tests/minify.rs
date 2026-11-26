use rs_js::{JsError, minify};

mod support;

use support::assert_golden;

#[test]
fn minify_javascript_matches_the_production_golden() {
    let source = r#"
function boot() {
  const message = "hello";
  const answer = 40 + 2;
  return { message, answer };
}
window.runtime = boot();
"#;
    let output = minify("runtime.js", source)
        .javascript()
        .run()
        .expect("minify should succeed");
    let surface = format!(
        "flavor={}\nmangled={}\ncontains_eval={}\ncontains_function_ctor={}\ninput_bytes={}\noutput_bytes={}\ncode={}\n",
        output.source_flavor.as_str(),
        output.mangling_applied,
        output.code.contains("eval("),
        output.code.contains("Function(") || output.code.contains("new Function("),
        output.input_bytes,
        output.output_bytes,
        output.code,
    );
    assert_golden("minify_surface.txt", &surface);
}

#[test]
fn minify_javascript_rejects_strict_csp_violations() {
    let err = minify("runtime.js", "eval('1+1');")
        .javascript()
        .run()
        .expect_err("strict CSP minify should reject eval");
    match err {
        JsError::CspViolation { .. } => {}
        other => panic!("expected csp violation, got {other}"),
    }
}

#[test]
fn minify_javascript_rejects_indirect_eval_strict_csp_violations() {
    let err = minify("runtime.js", "globalThis.eval('1+1');")
        .javascript()
        .run()
        .expect_err("strict CSP minify should reject indirect eval");
    match err {
        JsError::CspViolation { .. } => {}
        other => panic!("expected csp violation, got {other}"),
    }
}

#[test]
fn minify_javascript_is_deterministic() {
    let source = "const sum = (a, b) => a + b; window.value = sum(1, 2);";
    let first = minify("runtime.js", source)
        .javascript()
        .run()
        .expect("first minify should succeed");
    let second = minify("runtime.js", source)
        .javascript()
        .run()
        .expect("second minify should succeed");
    assert_eq!(first.code, second.code);
}
