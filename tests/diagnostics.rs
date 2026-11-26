use rs_js::{DiagnosticSeverity, JsError, minify};

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
