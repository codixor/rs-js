use rs_js::minify;

fn main() {
    match minify("runtime.js", "eval('1+1');").javascript().run() {
        Ok(output) => println!("{}", output.code),
        Err(err) => eprintln!("strict csp rejection: {err}"),
    }
}
