use rs_js::{DiagnosticSeverity, JsError, minify};

#[test]
fn early_errors_reach_analysis_and_minification() {
    for error in [
        rs_js::compat::analyze_source("invalid.mjs", "break;").unwrap_err(),
        minify("invalid.mjs", "break;").code().unwrap_err(),
    ] {
        let JsError::Semantic { file, diagnostics } = error else {
            panic!("expected a semantic diagnostic");
        };
        assert!(file.ends_with("invalid.mjs"));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.severity == DiagnosticSeverity::Error && !d.labels.is_empty())
        );
    }
    minify("valid.mjs", "while (true) { break; }")
        .code()
        .unwrap();
}

#[test]
fn parse_errors_expose_structured_diagnostics() {
    let err = minify("broken.js", "const = ;")
        .javascript()
        .run()
        .expect_err("parse error should surface");

    match err {
        JsError::Parse { file, diagnostics } => {
            assert!(file.ends_with("broken.js"));
            let first = diagnostics.first().expect("expected a parse diagnostic");
            assert_eq!(first.severity, DiagnosticSeverity::Error);
            assert_ne!(first.message, "");
        }
        other => panic!("expected parse error, got {other}"),
    }
}

#[test]
fn semantic_errors_expose_structured_diagnostics() {
    let err = minify("broken.js", "let value = 1; let value = 2;")
        .javascript()
        .run()
        .expect_err("semantic error should surface");

    match err {
        JsError::Semantic { file, diagnostics } => {
            assert!(file.ends_with("broken.js"));
            let first = diagnostics.first().expect("expected a semantic diagnostic");
            assert_eq!(first.severity, DiagnosticSeverity::Error);
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| !diagnostic.labels.is_empty()),
                "semantic diagnostics should preserve labels"
            );
        }
        other => panic!("expected semantic error, got {other}"),
    }
}
