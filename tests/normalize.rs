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
fn module_only_syntax_is_rejected_even_when_the_script_parse_is_ambiguous() {
    for source in [
        "export function url() { return import.meta.url; }",
        "await Promise.resolve(1);",
        "export const result = await(1);",
        "{ await(1); }",
        "for await (const value of []) {}",
        "using resource = null;",
    ] {
        for error in [
            normalize_embeddable("runtime.mjs", source).unwrap_err(),
            rs_js::minify("runtime.mjs", source)
                .embeddable()
                .code()
                .unwrap_err(),
        ] {
            assert!(
                matches!(error, UiJsError::UnsupportedEmbeddableModuleSyntax { ref reasons, .. } if !reasons.is_empty()),
                "{source}: {error}"
            );
        }
    }
}

#[test]
fn script_grammar_allows_async_functions_and_dynamic_imports() {
    let source = "export const load = async () => await import('./local.js');";
    let code = rs_js::minify("runtime.mjs", source)
        .embeddable()
        .code()
        .unwrap();
    assert!(code.contains("load"));
    assert!(code.contains("import("));
}

#[test]
fn typed_input_must_have_the_same_meaning_as_classic_javascript() {
    for source in [
        "export const value = transform<number>(1);",
        "export function boot(value: number) { return value; }",
    ] {
        assert!(matches!(
            normalize_embeddable("runtime.ts", source),
            Err(UiJsError::UnsupportedEmbeddableModuleSyntax { .. })
        ));
    }
    assert!(normalize_embeddable("runtime.js", "var await = x => x; await(1);").is_ok());
}

#[test]
fn type_only_imports_and_exports_can_leave_plain_script_code() {
    let source = "import type { Value } from './types'; export type { Value }; export function boot() { return 1; }";
    let code = rs_js::minify("runtime.ts", source)
        .embeddable()
        .code()
        .unwrap();
    assert!(code.contains("function boot("));
    assert!(!code.contains("import") && !code.contains("export"));
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
