use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process,
};

use rs_js::{CspMode, JsError, Mode, ObfuscatorOptions, SourceFlavor, process_source};

#[derive(Debug)]
struct Cli {
    input: PathBuf,
    output: Option<PathBuf>,
    options: ObfuscatorOptions,
}

fn print_usage(mut w: impl Write) -> io::Result<()> {
    writeln!(w, "cargo run --example cli -- <input> [options]")?;
    writeln!(w)?;
    writeln!(w, "Options:")?;
    writeln!(w, "  -o, --output <FILE>      Write output file")?;
    writeln!(
        w,
        "      --minify-only        Compress and emit minified code, no mangling"
    )?;
    writeln!(
        w,
        "      --analyze            Parse + semantic + traverse analysis only"
    )?;
    writeln!(
        w,
        "      --type <KIND>        One of: js, jsx, ts, tsx, mjs, cjs"
    )?;
    writeln!(
        w,
        "      --csp <MODE>         One of: off, compatible, strict"
    )?;
    writeln!(
        w,
        "      --double-quote       Prefer double quotes in emitted code"
    )?;
    writeln!(
        w,
        "      --allow-risky-mangle Keep mangling enabled even with dynamic-scope or CSP risks"
    )?;
    writeln!(w, "  -h, --help               Show help")?;
    Ok(())
}

fn parse_cli() -> Result<Cli, JsError> {
    let mut args = env::args_os();
    let _bin = args.next();

    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut options = ObfuscatorOptions::default();

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "-h" | "--help" => {
                let _ = print_usage(io::stdout());
                process::exit(0);
            }
            "-o" | "--output" => {
                let Some(value) = args.next() else {
                    return Err(JsError::InvalidArguments(
                        "missing value for --output".to_string(),
                    ));
                };
                output = Some(PathBuf::from(value));
            }
            "--minify-only" => options.mode = Mode::Minify,
            "--analyze" => options.mode = Mode::Analyze,
            "--double-quote" => options.single_quote = false,
            "--allow-risky-mangle" => options.allow_risky_mangle = true,
            "--type" => {
                let Some(value) = args.next() else {
                    return Err(JsError::InvalidArguments(
                        "missing value for --type".to_string(),
                    ));
                };
                let value = value.to_string_lossy();
                options.source_flavor = SourceFlavor::parse(&value);
                if options.source_flavor.is_none() {
                    return Err(JsError::InvalidArguments(format!(
                        "unsupported --type value: {value}"
                    )));
                }
            }
            "--csp" => {
                let Some(value) = args.next() else {
                    return Err(JsError::InvalidArguments(
                        "missing value for --csp".to_string(),
                    ));
                };
                let value = value.to_string_lossy();
                let Some(mode) = CspMode::parse(&value) else {
                    return Err(JsError::InvalidArguments(format!(
                        "unsupported --csp value: {value}"
                    )));
                };
                options.csp_mode = mode;
            }
            value if value.starts_with('-') => {
                return Err(JsError::InvalidArguments(format!("unknown flag: {value}")));
            }
            value => {
                if input.is_some() {
                    return Err(JsError::InvalidArguments(format!(
                        "unexpected extra positional argument: {value}"
                    )));
                }
                input = Some(PathBuf::from(value));
            }
        }
    }

    let Some(input) = input else {
        return Err(JsError::InvalidArguments("missing input file".to_string()));
    };

    Ok(Cli {
        input,
        output,
        options,
    })
}

fn write_output(path: Option<&Path>, code: &str) -> Result<(), JsError> {
    if let Some(path) = path {
        fs::write(path, code)?;
        Ok(())
    } else {
        io::stdout().write_all(code.as_bytes())?;
        Ok(())
    }
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(err) => {
            eprintln!("error: {err}");
            process::exit(1);
        }
    }
}

fn run() -> Result<(), JsError> {
    let cli = parse_cli()?;
    let input_text = fs::read_to_string(&cli.input)?;
    let result = process_source(&cli.input, &input_text, &cli.options)?;

    if matches!(cli.options.mode, Mode::Analyze) {
        eprintln!("file: {}", result.report.file_name);
        eprintln!("source flavor: {}", result.source_flavor.as_str());
        eprintln!("fingerprint: {:016x}", result.report.fingerprint);
        eprintln!("input bytes: {}", result.input_bytes);
        eprintln!("output bytes: {}", result.output_bytes);
        eprintln!("bindings: {}", result.report.bindings);
        eprintln!("identifier refs: {}", result.report.identifier_references);
        eprintln!("functions: {}", result.report.functions);
        eprintln!("classes: {}", result.report.classes);
        eprintln!("string literals: {}", result.report.string_literals);
        eprintln!("numeric literals: {}", result.report.numeric_literals);
        eprintln!("imports: {}", result.report.imports);
        eprintln!("exports: {}", result.report.exports);
        eprintln!("dynamic imports: {}", result.report.dynamic_imports);
        eprintln!("direct eval calls: {}", result.report.direct_eval_calls);
        eprintln!("indirect eval calls: {}", result.report.indirect_eval_calls);
        eprintln!(
            "Function-style constructors: {}",
            result.report.function_constructor_uses
        );
        eprintln!(
            "string timer callbacks: {}",
            result.report.string_timer_callbacks
        );
        eprintln!("with statements: {}", result.report.with_statements);
        eprintln!("debugger statements: {}", result.report.debugger_statements);
        eprintln!("jsx nodes: {}", result.report.jsx_nodes);
        for warning in result.report.warning_messages() {
            eprintln!("warning: {warning}");
        }
        return Ok(());
    }

    for warning in result.report.warning_messages() {
        eprintln!("warning: {warning}");
    }

    if !result.mangling_applied && matches!(cli.options.mode, Mode::Obfuscate) {
        eprintln!("warning: mangling was skipped due to conservative risk analysis");
    }

    write_output(cli.output.as_deref(), &result.code)
}
