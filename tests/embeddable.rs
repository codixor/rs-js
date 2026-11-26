use rs_js::minify;

mod support;

use support::assert_golden;

#[test]
fn host_can_call_embedded_bindings_after_minification_and_mangling() {
    use std::{io::ErrorKind, process::Command};

    let source = "export default function boot(input) { return input + 1; }
                  export class Reader { read() { return 2; } }
                  export const offset = 39;";
    for builder in [
        minify("host.mjs", source).embeddable(),
        minify("host.mjs", source).embeddable().obfuscate(),
    ] {
        let code = builder.code().unwrap();
        for binding in ["boot", "Reader", "offset"] {
            assert!(code.contains(binding), "{binding} missing from {code}");
        }
        let result = Command::new("node")
            .arg("-e")
            .arg(
                r"
            const vm = require('node:vm');
            const host = vm.createContext({});
            vm.runInContext(process.argv[1], host);
            if (vm.runInContext('boot(new Reader().read() + offset)', host) !== 42) process.exit(1);
        ",
            )
            .arg(&code)
            .output();
        match result {
            Ok(result) => assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            ),
            Err(err) if err.kind() == ErrorKind::NotFound => (),
            Err(err) => panic!("could not run Node: {err}"),
        }
    }
}

#[test]
fn minify_embeddable_matches_the_production_golden() {
    let source = r#"
export default function boot() {
  const label = "hello";
  return { label, value: 42 };
}
window.runtime = boot();
"#;
    let output = minify("runtime.mjs", source)
        .module()
        .embeddable()
        .run()
        .expect("embeddable minify works");
    let surface = format!(
        "flavor={}\nmangled={}\ninput_bytes={}\noutput_bytes={}\ncode={}\n",
        output.source_flavor.as_str(),
        output.mangling_applied,
        output.input_bytes,
        output.output_bytes,
        output.code,
    );
    assert_golden("embeddable_minify_surface.txt", &surface);
}

#[test]
fn obfuscate_source_matches_the_production_golden() {
    let source = r#"
function boot(answer) {
  const message = "hello";
  return { message, answer };
}
window.runtime = boot(42);
"#;
    let output = minify("runtime.js", source)
        .javascript()
        .compatible_csp()
        .obfuscate()
        .run()
        .expect("obfuscate should succeed");
    let surface = format!(
        "flavor={}\nmangled={}\noutput_bytes={}\ncode={}\n",
        output.source_flavor.as_str(),
        output.mangling_applied,
        output.output_bytes,
        output.code,
    );
    assert_golden("obfuscate_surface.txt", &surface);
}

#[test]
fn obfuscate_source_skips_mangling_for_indirect_eval_patterns() {
    let source = "globalThis.eval('40 + 2'); const value = 42; window.answer = value;";
    let output = minify("runtime.js", source)
        .javascript()
        .compatible_csp()
        .obfuscate()
        .run()
        .expect("compatible obfuscation should still work");

    assert!(!output.mangling_applied);
    assert!(output.report.indirect_eval_calls >= 1);
}
