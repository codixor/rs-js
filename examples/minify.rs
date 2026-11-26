use rs_js::minify;

fn main() -> Result<(), rs_js::JsError> {
    let output = minify(
        "runtime.js",
        "function boot(){const answer=40+2;return answer}window.runtime=boot();",
    )
    .javascript()
    .run()?;
    println!("{}", output.code);
    Ok(())
}
