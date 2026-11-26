use rs_js::minify;

fn main() -> Result<(), rs_js::JsError> {
    let output = minify(
        "runtime.mjs",
        "export default function boot(){ return 42; } window.runtime = boot();",
    )
    .embeddable()
    .run()?;
    println!("{}", output.code);
    Ok(())
}
