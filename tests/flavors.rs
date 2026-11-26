use std::path::Path;

use rs_js::{Mode, ObfuscatorOptions, SourceFlavor, minify, process_source};

#[test]
fn analyze_source_accepts_js_and_module_flavors() {
    let js = minify("runtime.js", "const answer = 40 + 2;")
        .javascript()
        .compatible_csp()
        .analyze()
        .report()
        .expect("js should parse");
    let mjs = minify("runtime.mjs", "export const answer = 40 + 2;")
        .module()
        .compatible_csp()
        .analyze()
        .report()
        .expect("mjs should parse");
    let cjs = minify("runtime.cjs", "module.exports = { answer: 42 };")
        .commonjs()
        .compatible_csp()
        .analyze()
        .report()
        .expect("cjs should parse");

    assert_eq!(js.direct_eval_calls, 0);
    assert_eq!(mjs.exports, 1);
    assert_eq!(cjs.exports, 0);
}

#[test]
fn analyze_source_accepts_jsx_ts_and_tsx() {
    let jsx = minify(
        "component.jsx",
        "export function View(){ return <section>Hello</section>; }",
    )
    .jsx()
    .compatible_csp()
    .analyze()
    .report()
    .expect("jsx should parse");
    let ts = minify(
        "runtime.ts",
        "export function boot(value: number): number { return value + 1; }",
    )
    .typescript()
    .compatible_csp()
    .analyze()
    .report()
    .expect("ts should parse");
    let tsx = minify(
        "component.tsx",
        "export function View(): JSX.Element { return <section>Hello</section>; }",
    )
    .tsx()
    .compatible_csp()
    .analyze()
    .report()
    .expect("tsx should parse");

    assert!(jsx.jsx_nodes >= 1);
    assert_eq!(ts.jsx_nodes, 0);
    assert!(tsx.jsx_nodes >= 1);
}

#[test]
fn minify_source_handles_tsx_input() {
    let output = minify(
        "component.tsx",
        "export function View(): JSX.Element { return <section>Hello</section>; }",
    )
    .tsx()
    .run()
    .expect("tsx minify should succeed");

    assert_ne!(output.code, "");
}

#[test]
fn source_flavor_path_inference_is_explicit() {
    assert_eq!(
        SourceFlavor::from_path(Path::new("runtime.js")),
        Some(SourceFlavor::Js)
    );
    assert_eq!(
        SourceFlavor::from_path(Path::new("runtime.mjs")),
        Some(SourceFlavor::Mjs)
    );
    assert_eq!(
        SourceFlavor::from_path(Path::new("runtime.cjs")),
        Some(SourceFlavor::Cjs)
    );
    assert_eq!(
        SourceFlavor::from_path(Path::new("component.jsx")),
        Some(SourceFlavor::Jsx)
    );
    assert_eq!(
        SourceFlavor::from_path(Path::new("runtime.ts")),
        Some(SourceFlavor::Ts)
    );
    assert_eq!(
        SourceFlavor::from_path(Path::new("component.tsx")),
        Some(SourceFlavor::Tsx)
    );
    assert_eq!(SourceFlavor::from_path(Path::new("runtime.txt")), None);
}

#[test]
fn explicit_source_flavor_override_handles_unknown_extensions() {
    let source = "export const answer: number = 42;";
    let options = ObfuscatorOptions::analyze().with_source_flavor(SourceFlavor::Ts);
    let output = process_source(Path::new("inline.module"), source, &options)
        .expect("explicit source flavor should override path inference");

    assert_eq!(output.source_flavor, SourceFlavor::Ts);
    assert_eq!(output.input_bytes, source.len());
    assert_eq!(output.output_bytes, source.len());
}

#[test]
fn explicit_option_constructors_keep_the_lower_level_surface_intentional() {
    let options = ObfuscatorOptions::minify()
        .with_source_flavor(SourceFlavor::Js)
        .with_csp_mode(rs_js::CspMode::Compatible)
        .with_single_quote(false)
        .with_parse_regular_expression(true)
        .with_allow_risky_mangle(true);

    assert_eq!(options.mode, Mode::Minify);
    assert_eq!(options.source_flavor, Some(SourceFlavor::Js));
    assert_eq!(options.csp_mode, rs_js::CspMode::Compatible);
    assert!(!options.single_quote);
    assert!(options.parse_regular_expression);
    assert!(options.allow_risky_mangle);
}
