use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use rs_js::{
    CspMode,
    lazy_bundle::{LazyBundleBuilder, LazyBundleError, LazyBundleMinifyLevel},
};

struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires Bun on PATH"]
fn cache_tracks_settings_missing_output_and_failed_rebuilds() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir =
        TempDir(std::env::temp_dir().join(format!("rs-js-lazy-{}-{unique}", std::process::id())));
    fs::create_dir(&dir.0).unwrap();
    let input = dir.0.join("entry.ts");
    let output = dir.0.join("bundle.js");
    fs::write(
        &input,
        "export function twice(num: number) { return num * 2; }",
    )
    .unwrap();
    let bundle = |level, csp| {
        LazyBundleBuilder::new()
            .with_binding(&input, &output)
            .with_minify_level(level)
            .with_csp_mode(csp)
            .run()
    };

    bundle(LazyBundleMinifyLevel::None, CspMode::Off).unwrap();
    let original = fs::read_to_string(&output).unwrap();
    bundle(LazyBundleMinifyLevel::Syntax, CspMode::Off).unwrap();
    let minified = fs::read_to_string(&output).unwrap();
    assert_ne!(original, minified);
    fs::remove_file(&output).unwrap();
    bundle(LazyBundleMinifyLevel::Syntax, CspMode::Off).unwrap();
    assert_eq!(fs::read_to_string(&output).unwrap(), minified);

    fs::write(&input, "setTimeout('sample', 5);").unwrap();
    bundle(LazyBundleMinifyLevel::Syntax, CspMode::Off).unwrap();
    let prior = fs::read_to_string(&output).unwrap();
    assert!(matches!(
        bundle(LazyBundleMinifyLevel::Syntax, CspMode::Strict),
        Err(LazyBundleError::Minify { .. })
    ));
    bundle(LazyBundleMinifyLevel::Syntax, CspMode::Off).unwrap();
    assert_eq!(fs::read_to_string(&output).unwrap(), prior);
}
