use rs_js::minify;

fn main() -> Result<(), rs_js::JsError> {
    let report = minify("input.js", "const answer = () => 40 + 2;")
        .javascript()
        .analyze()
        .report()?;
    println!(
        "fingerprint={:016x} bindings={} warnings={}",
        report.fingerprint,
        report.bindings,
        report.warning_messages().len()
    );
    Ok(())
}
