use rs_js::minify;

mod support;

use support::assert_golden;

#[test]
fn analyze_source_matches_the_production_golden() {
    let source = r#"
export function boot() {
  debugger;
  setTimeout("boot()", 10);
  return eval("40 + 2");
}
"#;
    let report = minify("runtime.mjs", source)
        .module()
        .compatible_csp()
        .analyze()
        .report()
        .expect("analyze should succeed");
    let warnings = report.warning_messages().join(" | ");
    let surface = format!(
        "file={}\nfingerprint={:016x}\nbindings={}\nfunctions={}\nexports={}\ndirect_eval_calls={}\nindirect_eval_calls={}\nstring_timer_callbacks={}\ndebugger_statements={}\nwarning_count={}\nwarnings={}\n",
        report.file_name,
        report.fingerprint,
        report.bindings,
        report.functions,
        report.exports,
        report.direct_eval_calls,
        report.indirect_eval_calls,
        report.string_timer_callbacks,
        report.debugger_statements,
        report.warning_messages().len(),
        warnings,
    );
    assert_golden("analysis_surface.txt", &surface);
}

#[test]
fn analyze_source_reports_function_constructor_as_a_strict_csp_violation() {
    let source = "const build = new Function('return 42'); window.answer = build();";
    let report = minify("runtime.js", source)
        .javascript()
        .compatible_csp()
        .analyze()
        .report()
        .expect("analyze should succeed");

    assert!(report.function_constructor_uses >= 1);
    assert!(report.has_strict_csp_violations());
    assert!(
        report
            .warning_messages()
            .iter()
            .any(|message| message.contains("Function-style constructor"))
    );
}

#[test]
fn analyze_source_reports_indirect_eval_patterns() {
    let source = "const left = (0, eval)('21 + 21'); const right = globalThis.eval('40 + 2');";
    let report = minify("runtime.js", source)
        .javascript()
        .compatible_csp()
        .analyze()
        .report()
        .expect("analyze should succeed");

    assert!(report.indirect_eval_calls >= 2);
    assert!(report.has_strict_csp_violations());
    assert!(
        report
            .warning_messages()
            .iter()
            .any(|message| message.contains("indirect eval pattern"))
    );
}
