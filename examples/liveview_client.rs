use rs_js::minify;

const LIVEVIEW_CLIENT: &str = include_str!("../assets/liveview_client.js");

fn main() -> Result<(), rs_js::JsError> {
    let output = minify("liveview_client.js", LIVEVIEW_CLIENT)
        .javascript()
        .run()?;

    eprintln!("flavor: {}", output.source_flavor.as_str());
    eprintln!("input bytes: {}", output.input_bytes);
    eprintln!("output bytes: {}", output.output_bytes);
    eprintln!("warnings: {}", output.report.warning_messages().len());
    eprintln!(
        "strict csp safe: {}",
        !output.report.has_strict_csp_violations()
    );

    println!("{}", output.code);
    Ok(())
}
