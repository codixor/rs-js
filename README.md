# rs-js

A Rust library for analyzing and minifying JavaScript and TypeScript with Oxc. It accepts JS, MJS, CJS, JSX, TS, and TSX. You can also use it to check for common Content Security Policy (CSP) violations, mangle identifiers, or prepare module snippets for embedding in a runtime.

## Quick start

With Rust 1.100 or newer, add the library to your project.

```toml
[dependencies]
rs-js = "0.1"
```

Pass a source name and its text to `minify`. The extension selects the syntax. The builder defaults to strict CSP checking and emits minified code with identifier mangling disabled.

```rust
use rs_js::{JsError, minify};

fn main() -> Result<(), JsError> {
    let output = minify("runtime.js", "window.answer = 40 + 2;").run()?;
    assert_eq!(output.code, "window.answer=42;");
    assert!(!output.mangling_applied);
    println!("{}", output.code);
    Ok(())
}
```

`run()` returns the code, analysis report, source flavor, input and output byte counts, and whether mangling ran. Use `.code()` when you only need the emitted source. When an input has no recognized extension, specify its syntax with `.javascript()`, `.module()`, `.commonjs()`, `.jsx()`, `.typescript()`, or `.tsx()`.

## Check CSP findings

Strict mode returns `JsError::CspViolation` for recognized uses of direct or indirect `eval`, string-compiling function constructors, and string timer callbacks. To inspect findings before deciding what to do with the source, select compatible mode and request an analysis report.

```rust
use rs_js::{JsError, minify};

fn main() -> Result<(), JsError> {
    let source = "setTimeout('refresh()', 1000);";
    let report = minify("timer.js", source).compatible_csp().analyze().report()?;
    assert!(report.has_strict_csp_violations());
    assert!(report.string_timer_callbacks > 0);
    Ok(())
}
```

The report includes syntax counts, a fingerprint of the counts and flags, and warnings about features such as dynamic imports, `debugger`, and `with`. CSP detection uses syntax and text patterns, so review findings in context and enforce the actual policy in the browser. Parse and semantic errors carry structured diagnostics with messages, severity and byte-range labels.

## Transformations

- `.obfuscate()` enables identifier mangling. Dynamic-scope patterns can change name resolution at runtime, so the pipeline skips mangling for these inputs and records the finding. `.allow_risky_mangle()` overrides that decision.
- `.embeddable()` removes type-only import/export declarations and rewrites supported exports into local declarations before minifying. Value imports, value re-exports, anonymous default exports and TypeScript export assignments are among the forms that return `JsError::UnsupportedEmbeddableModuleSyntax`. Call `normalize_embeddable` for just the normalization step.
- `process_source` accepts a path and `ObfuscatorOptions` for callers that supply configuration directly. Convenience functions are available in `rs_js::compat`.
- The optional build-script helper `rs_js::lazy_bundle::LazyBundleBuilder` invokes Bun to bundle and transpile assets and can post-process the result with the Rust minifier. Only callers of this helper need a `bun` executable on their path.

## Examples and tests

From a checkout, these commands run the library examples against local inputs. The CLI example defaults to compatible CSP mode and identifier mangling. Use `--minify-only --csp strict` for the same settings as the library quick start.

```bash
cargo run --locked --example minify
cargo run --locked --example cli -- assets/liveview_client.js --analyze
cargo run --locked --example cli -- assets/liveview_client.js --minify-only --csp strict -o target/liveview.min.js
```

The `liveview_client` example writes the minified Dioxus bridge to stdout and its analysis summary to stderr. The golden tests check its size and output hash along with syntax handling, diagnostics, normalization, and mangling behavior.

```bash
bash scripts/validate.sh
cargo bench --locked --bench pipeline_core
```

When deliberately changing output, use `UPDATE_GOLDENS=1 cargo test --locked`, inspect the changes under `tests/golden/`, then run `cargo test --locked` with the variable unset.

## License

Distributed under the MIT License or the Apache License, Version 2.0, at your option. The bridge fixtures carry a separate Dioxus attribution. [MIT](https://github.com/codixor/rs-js/blob/main/LICENSE-MIT) · [Apache-2.0](https://github.com/codixor/rs-js/blob/main/LICENSE-APACHE) · [Third-party notices](https://github.com/codixor/rs-js/blob/main/THIRD-PARTY-NOTICES.md)
