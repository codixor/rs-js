use rs_js::{UiJsError, normalize_embeddable};

mod support;

use support::assert_golden;

#[test]
fn normalize_embeddable_matches_the_production_golden() {
    let source = r#"
import type { Foo } from "./foo";
export type { Foo };
export function boot() {
  return 42;
}
"#;
    let normalized = normalize_embeddable("runtime.ts", source).expect("normalize should succeed");
    let surface = format!(
        "contains_import={}\ncontains_export={}\noutput_len={}\ncode={}\n",
        normalized.contains("import "),
        normalized.contains("export "),
        normalized.len(),
        normalized,
    );
    assert_golden("normalize_surface.txt", &surface);
}

#[test]
fn normalize_embeddable_rejects_unsupported_reexports() {
    let err = normalize_embeddable("runtime.mjs", "export { boot } from './other.js';")
        .expect_err("re-export should be rejected");
    match err {
        UiJsError::UnsupportedEmbeddableModuleSyntax { .. } => {}
        other => panic!("expected unsupported embeddable syntax, got {other}"),
    }
}

#[test]
fn normalize_embeddable_rejects_typescript_export_assignment() {
    let err = normalize_embeddable("runtime.ts", "export = boot;")
        .expect_err("export assignment should be rejected");
    match err {
        UiJsError::UnsupportedEmbeddableModuleSyntax { reasons, .. } => {
            assert!(
                reasons
                    .iter()
                    .any(|reason| reason.contains("export assignment"))
            );
        }
        other => panic!("expected unsupported embeddable syntax, got {other}"),
    }
}

#[test]
fn normalize_embeddable_rejects_typescript_namespace_export() {
    let err = normalize_embeddable("runtime.ts", "export as namespace Runtime;")
        .expect_err("namespace export should be rejected");
    match err {
        UiJsError::UnsupportedEmbeddableModuleSyntax { reasons, .. } => {
            assert!(
                reasons
                    .iter()
                    .any(|reason| reason.contains("namespace export"))
            );
        }
        other => panic!("expected unsupported embeddable syntax, got {other}"),
    }
}
