use std::path::Path;

use oxc::{
    allocator::Allocator,
    ast::ast::{
        ExportDefaultDeclaration, ExportDefaultDeclarationKind, ExportNamedDeclaration,
        ImportDeclaration, ImportOrExportKind, Program, Statement,
    },
    codegen::{Codegen, CodegenOptions},
    mangler::{MangleOptions, Mangler},
    minifier::{CompressOptions, Compressor},
    parser::{ParseOptions, Parser},
    semantic::{Scoping, Semantic, SemanticBuilder},
    span::{ContentEq, GetSpan, SourceType},
};
use oxc_traverse::traverse_mut;

use crate::{
    analysis::{AnalysisReport, Analyzer},
    error::{JsDiagnostic, UiJsError},
    options::{CspMode, Mode, ObfuscatorOptions, SourceFlavor},
};

#[derive(Debug, Clone)]
pub struct ProcessedOutput {
    pub code: String,
    pub report: AnalysisReport,
    pub source_flavor: SourceFlavor,
    pub input_bytes: usize,
    pub output_bytes: usize,
    pub mangling_applied: bool,
}

#[derive(Debug, Clone)]
pub struct MinifyBuilder<'a> {
    source_name: &'a str,
    source_text: &'a str,
    options: ObfuscatorOptions,
    normalize_embeddable: bool,
}

impl<'a> MinifyBuilder<'a> {
    #[must_use]
    pub fn new(source_name: &'a str, source_text: &'a str) -> Self {
        Self {
            source_name,
            source_text,
            options: ObfuscatorOptions {
                mode: Mode::Minify,
                csp_mode: CspMode::Strict,
                ..ObfuscatorOptions::default()
            },
            normalize_embeddable: false,
        }
    }

    #[must_use]
    pub fn source_flavor(mut self, flavor: SourceFlavor) -> Self {
        self.options.source_flavor = Some(flavor);
        self
    }

    #[must_use]
    pub fn javascript(self) -> Self {
        self.source_flavor(SourceFlavor::Js)
    }

    #[must_use]
    pub fn jsx(self) -> Self {
        self.source_flavor(SourceFlavor::Jsx)
    }

    #[must_use]
    pub fn typescript(self) -> Self {
        self.source_flavor(SourceFlavor::Ts)
    }

    #[must_use]
    pub fn tsx(self) -> Self {
        self.source_flavor(SourceFlavor::Tsx)
    }

    #[must_use]
    pub fn module(self) -> Self {
        self.source_flavor(SourceFlavor::Mjs)
    }

    #[must_use]
    pub fn commonjs(self) -> Self {
        self.source_flavor(SourceFlavor::Cjs)
    }

    #[must_use]
    pub fn csp_mode(mut self, mode: CspMode) -> Self {
        self.options.csp_mode = mode;
        self
    }

    #[must_use]
    pub fn strict_csp(self) -> Self {
        self.csp_mode(CspMode::Strict)
    }

    #[must_use]
    pub fn compatible_csp(self) -> Self {
        self.csp_mode(CspMode::Compatible)
    }

    #[must_use]
    pub fn without_csp(self) -> Self {
        self.csp_mode(CspMode::Off)
    }

    #[must_use]
    pub fn double_quote(mut self) -> Self {
        self.options.single_quote = false;
        self
    }

    #[must_use]
    pub fn single_quote(mut self) -> Self {
        self.options.single_quote = true;
        self
    }

    #[must_use]
    pub fn parse_regular_expression(mut self, yes: bool) -> Self {
        self.options.parse_regular_expression = yes;
        self
    }

    #[must_use]
    pub fn allow_risky_mangle(mut self) -> Self {
        self.options.allow_risky_mangle = true;
        self
    }

    #[must_use]
    pub fn obfuscate(mut self) -> Self {
        self.options.mode = Mode::Obfuscate;
        self
    }

    #[must_use]
    pub fn analyze(mut self) -> Self {
        self.options.mode = Mode::Analyze;
        self
    }

    #[must_use]
    pub fn embeddable(mut self) -> Self {
        self.normalize_embeddable = true;
        self
    }

    /// Runs the configured JS pipeline.
    ///
    /// # Errors
    ///
    /// Returns an error if normalization, parsing, semantic analysis, or CSP
    /// validation fails.
    pub fn run(self) -> Result<ProcessedOutput, UiJsError> {
        process(
            Path::new(self.source_name),
            self.source_text,
            &self.options,
            self.normalize_embeddable,
        )
    }

    /// Runs the configured JS pipeline and returns only the analysis report.
    ///
    /// # Errors
    ///
    /// Returns an error if the configured pipeline fails.
    pub fn report(self) -> Result<AnalysisReport, UiJsError> {
        Ok(self.run()?.report)
    }

    /// Runs the configured JS pipeline and returns only the emitted code.
    ///
    /// # Errors
    ///
    /// Returns an error if the configured pipeline fails.
    pub fn code(self) -> Result<String, UiJsError> {
        Ok(self.run()?.code)
    }
}

#[must_use]
pub fn minify<'a>(source_name: &'a str, source_text: &'a str) -> MinifyBuilder<'a> {
    MinifyBuilder::new(source_name, source_text)
}

#[derive(Debug, Clone)]
struct TextEdit {
    start: usize,
    end: usize,
    replacement: String,
}

/// Parses, analyzes, and optionally minifies or mangles a source file.
///
/// # Errors
///
/// Returns an error if parsing, semantic analysis, CSP validation, or
/// embeddable normalization fails.
pub fn process_source(
    source_path: &Path,
    source_text: &str,
    options: &ObfuscatorOptions,
) -> Result<ProcessedOutput, UiJsError> {
    process(source_path, source_text, options, false)
}

fn process(
    source_path: &Path,
    source_text: &str,
    options: &ObfuscatorOptions,
    embeddable: bool,
) -> Result<ProcessedOutput, UiJsError> {
    let allocator = Allocator::default();
    let source_flavor = options.source_flavor_for_path(source_path)?;
    let normalized;
    let (mut program, scoping) = if embeddable {
        let source_type;
        (normalized, source_type) = rewrite_embeddable_source(
            source_path,
            source_text,
            source_flavor,
            options.parse_regular_expression,
        )?;
        parse_embeddable_script(
            &allocator,
            source_path,
            &normalized,
            source_type,
            options.parse_regular_expression,
        )?
    } else {
        let program = parse_program(
            &allocator,
            source_path,
            source_text,
            source_flavor.to_source_type(),
            options.parse_regular_expression,
        )?;
        let scoping = build_semantic(source_path, &program)?.into_scoping();
        (program, scoping)
    };
    let source_text = program.source_text;
    let report = analyze_program(&allocator, source_path, source_text, &mut program, scoping);
    enforce_csp_mode(source_path, &report, options.csp_mode)?;
    let input_bytes = source_text.len();

    if !options.should_minify() {
        return Ok(ProcessedOutput {
            code: source_text.to_owned(),
            report,
            source_flavor,
            input_bytes,
            output_bytes: input_bytes,
            mangling_applied: false,
        });
    }

    let (code, mangling_applied) = emit_program(&allocator, &mut program, &report, options);
    let output_bytes = code.len();

    Ok(ProcessedOutput {
        code,
        report,
        source_flavor,
        input_bytes,
        output_bytes,
        mangling_applied,
    })
}

/// Analyzes source using the flavor inferred from its file name.
///
/// # Errors
///
/// Returns an error if the source flavor is unsupported or analysis fails.
pub fn analyze_source(source_name: &str, source_text: &str) -> Result<AnalysisReport, UiJsError> {
    minify(source_name, source_text)
        .compatible_csp()
        .analyze()
        .report()
}

/// Analyzes a JavaScript source string with the default JS flavor.
///
/// # Errors
///
/// Returns an error if parsing or semantic analysis fails.
pub fn analyze_javascript(
    source_name: &str,
    source_text: &str,
) -> Result<AnalysisReport, UiJsError> {
    minify(source_name, source_text)
        .javascript()
        .compatible_csp()
        .analyze()
        .report()
}

/// Obfuscates a JavaScript source string with the crate's default
/// compatibility settings.
///
/// # Errors
///
/// Returns an error if parsing, semantic analysis, or CSP compatibility checks
/// fail.
pub fn obfuscate_source(
    source_name: &str,
    source_text: &str,
) -> Result<ProcessedOutput, UiJsError> {
    minify(source_name, source_text)
        .compatible_csp()
        .obfuscate()
        .run()
}

/// Obfuscates a JavaScript source string with the crate's default JS flavor
/// and compatibility settings.
///
/// # Errors
///
/// Returns an error if parsing, semantic analysis, or CSP compatibility checks
/// fail.
pub fn obfuscate_javascript(
    source_name: &str,
    source_text: &str,
) -> Result<ProcessedOutput, UiJsError> {
    minify(source_name, source_text)
        .javascript()
        .compatible_csp()
        .obfuscate()
        .run()
}

/// Minifies a JavaScript source string using strict CSP validation.
///
/// # Errors
///
/// Returns an error if parsing, semantic analysis, or CSP validation fails.
pub fn minify_source(source_name: &str, source_text: &str) -> Result<ProcessedOutput, UiJsError> {
    minify(source_name, source_text).run()
}

/// Minifies a JavaScript source string using strict CSP validation.
///
/// # Errors
///
/// Returns an error if parsing, semantic analysis, or CSP validation fails.
pub fn minify_javascript(
    source_name: &str,
    source_text: &str,
) -> Result<ProcessedOutput, UiJsError> {
    minify(source_name, source_text).javascript().run()
}

/// Normalizes embeddable module syntax into a plain source string.
///
/// # Errors
///
/// Returns an error if the input cannot be parsed or contains unsupported
/// embeddable module syntax.
pub fn normalize_embeddable(source_name: &str, source_text: &str) -> Result<String, UiJsError> {
    let path = Path::new(source_name);
    let flavor = SourceFlavor::from_path(path).unwrap_or(SourceFlavor::Mjs);
    let (normalized, source_type) = rewrite_embeddable_source(path, source_text, flavor, true)?;
    parse_embeddable_script(&Allocator::default(), path, &normalized, source_type, true)?;
    Ok(normalized)
}

/// Normalizes embeddable JavaScript module syntax into a plain source string.
///
/// # Errors
///
/// Returns an error if the input cannot be parsed or contains unsupported
/// embeddable module syntax.
pub fn normalize_embeddable_javascript(
    source_name: &str,
    source_text: &str,
) -> Result<String, UiJsError> {
    normalize_embeddable(source_name, source_text)
}

/// Normalizes and minifies embeddable JavaScript using strict CSP validation.
///
/// # Errors
///
/// Returns an error if normalization, parsing, semantic analysis, or CSP
/// validation fails.
pub fn minify_embeddable(
    source_name: &str,
    source_text: &str,
) -> Result<ProcessedOutput, UiJsError> {
    minify(source_name, source_text).embeddable().run()
}

/// Normalizes and minifies embeddable JavaScript using strict CSP validation.
///
/// # Errors
///
/// Returns an error if normalization, parsing, semantic analysis, or CSP
/// validation fails.
pub fn minify_embeddable_javascript(
    source_name: &str,
    source_text: &str,
) -> Result<ProcessedOutput, UiJsError> {
    minify(source_name, source_text)
        .javascript()
        .embeddable()
        .run()
}

fn rewrite_embeddable_source(
    source_path: &Path,
    source_text: &str,
    source_flavor: SourceFlavor,
    parse_regular_expression: bool,
) -> Result<(String, SourceType), UiJsError> {
    let allocator = Allocator::default();

    let program = parse_program(
        &allocator,
        source_path,
        source_text,
        source_flavor.to_source_type(),
        parse_regular_expression,
    )?;
    let mut edits = Vec::new();
    let mut reasons = Vec::new();

    for statement in &program.body {
        match statement {
            Statement::ImportDeclaration(decl) => {
                classify_import(decl, &mut edits, &mut reasons);
            }
            Statement::ExportNamedDeclaration(decl) => {
                classify_named_export(source_text, decl, &mut edits, &mut reasons);
            }
            Statement::ExportDefaultDeclaration(decl) => {
                classify_default_export(source_text, decl, &mut edits, &mut reasons);
            }
            Statement::ExportAllDeclaration(decl) => {
                reasons.push(format!(
                    "unsupported re-export syntax at byte range {}..{}",
                    decl.span.start, decl.span.end
                ));
            }
            Statement::TSExportAssignment(decl) => {
                let span = decl.span();
                reasons.push(format!(
                    "unsupported TypeScript export assignment at byte range {}..{}",
                    span.start, span.end
                ));
            }
            Statement::TSNamespaceExportDeclaration(decl) => {
                let span = decl.span();
                reasons.push(format!(
                    "unsupported TypeScript namespace export at byte range {}..{}",
                    span.start, span.end
                ));
            }
            _ => {}
        }
    }

    if !reasons.is_empty() {
        return Err(UiJsError::UnsupportedEmbeddableModuleSyntax {
            file: source_path.to_path_buf(),
            reasons,
        });
    }

    build_semantic(source_path, &program)?;
    Ok((
        apply_text_edits(source_text, edits.as_mut_slice())?,
        program.source_type,
    ))
}

fn parse_embeddable_script<'a>(
    allocator: &'a Allocator,
    source_path: &Path,
    source_text: &'a str,
    source_type: SourceType,
    parse_regular_expression: bool,
) -> Result<(Program<'a>, Scoping), UiJsError> {
    let unsupported = |err: UiJsError| UiJsError::UnsupportedEmbeddableModuleSyntax {
        file: source_path.to_path_buf(),
        reasons: err
            .diagnostics()
            .iter()
            .map(JsDiagnostic::summary_line)
            .collect(),
    };
    let program = parse_program(
        allocator,
        source_path,
        source_text,
        SourceType::script(),
        parse_regular_expression,
    )
    .map_err(unsupported)?;
    let scoping = build_semantic(source_path, &program)
        .map_err(unsupported)?
        .into_scoping();
    // Both goals can parse the same text differently: e.g. module `await(x)`
    // or a TypeScript generic call `f<T>(x)`. Compare the normalized bodies.
    if !(source_type.is_script() && source_type.is_javascript() && !source_type.is_jsx()) {
        let original = parse_program(
            allocator,
            source_path,
            source_text,
            source_type,
            parse_regular_expression,
        )
        .map_err(unsupported)?;
        if original.body.content_ne(&program.body) {
            return Err(UiJsError::UnsupportedEmbeddableModuleSyntax {
                file: source_path.to_path_buf(),
                reasons: vec!["normalized source has a different meaning in classic JavaScript script grammar".to_owned()],
            });
        }
    }
    Ok((program, scoping))
}

fn classify_import(
    decl: &ImportDeclaration<'_>,
    edits: &mut Vec<TextEdit>,
    reasons: &mut Vec<String>,
) {
    if decl.import_kind == ImportOrExportKind::Type {
        edits.push(TextEdit {
            start: decl.span.start as usize,
            end: decl.span.end as usize,
            replacement: String::new(),
        });
        return;
    }

    reasons.push(format!(
        "unsupported value import from '{}' at byte range {}..{}",
        decl.source.value.as_str(),
        decl.span.start,
        decl.span.end
    ));
}

fn classify_named_export(
    source_text: &str,
    decl: &ExportNamedDeclaration<'_>,
    edits: &mut Vec<TextEdit>,
    reasons: &mut Vec<String>,
) {
    if decl.export_kind == ImportOrExportKind::Type {
        edits.push(TextEdit {
            start: decl.span.start as usize,
            end: decl.span.end as usize,
            replacement: String::new(),
        });
        return;
    }

    if let Some(source) = &decl.source {
        reasons.push(format!(
            "unsupported re-export from '{}' at byte range {}..{}",
            source.value.as_str(),
            decl.span.start,
            decl.span.end
        ));
        return;
    }

    if let Some(declaration) = &decl.declaration {
        let span = declaration.span();
        edits.push(TextEdit {
            start: decl.span.start as usize,
            end: decl.span.end as usize,
            replacement: slice_span(source_text, span.start as usize, span.end as usize).to_owned(),
        });
        return;
    }

    edits.push(TextEdit {
        start: decl.span.start as usize,
        end: decl.span.end as usize,
        replacement: String::new(),
    });
}

fn classify_default_export(
    source_text: &str,
    decl: &ExportDefaultDeclaration<'_>,
    edits: &mut Vec<TextEdit>,
    reasons: &mut Vec<String>,
) {
    match &decl.declaration {
        ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
            if function.id.is_none() {
                reasons.push(format!(
                    "anonymous default function export is not safe to embed at byte range {}..{}",
                    decl.span.start, decl.span.end
                ));
                return;
            }

            let span = function.span();
            edits.push(TextEdit {
                start: decl.span.start as usize,
                end: decl.span.end as usize,
                replacement: slice_span(source_text, span.start as usize, span.end as usize).to_owned(),
            });
        }
        ExportDefaultDeclarationKind::ClassDeclaration(class) => {
            if class.id.is_none() {
                reasons.push(format!(
                    "anonymous default class export is not safe to embed at byte range {}..{}",
                    decl.span.start, decl.span.end
                ));
                return;
            }

            let span = class.span();
            edits.push(TextEdit {
                start: decl.span.start as usize,
                end: decl.span.end as usize,
                replacement: slice_span(source_text, span.start as usize, span.end as usize).to_owned(),
            });
        }
        ExportDefaultDeclarationKind::TSInterfaceDeclaration(interface_decl) => {
            edits.push(TextEdit {
                start: decl.span.start as usize,
                end: decl.span.end as usize,
                replacement: slice_span(
                    source_text,
                    interface_decl.span.start as usize,
                    interface_decl.span.end as usize,
                )
                .to_owned(),
            });
        }
        _ => reasons.push(format!(
            "default export expressions are not supported for embeddable bundles at byte range {}..{}",
            decl.span.start, decl.span.end
        )),
    }
}

fn apply_text_edits(source_text: &str, edits: &mut [TextEdit]) -> Result<String, UiJsError> {
    edits.sort_by_key(|edit| edit.start);

    let mut out = String::with_capacity(source_text.len());
    let mut cursor = 0usize;

    for edit in edits.iter() {
        if edit.start > edit.end || edit.end > source_text.len() {
            return Err(UiJsError::InvalidArguments(format!(
                "invalid edit range {}..{} for source of length {}",
                edit.start,
                edit.end,
                source_text.len()
            )));
        }

        if edit.start < cursor {
            return Err(UiJsError::InvalidArguments(format!(
                "overlapping edit range {}..{}",
                edit.start, edit.end
            )));
        }

        out.push_str(slice_span(source_text, cursor, edit.start));
        out.push_str(&edit.replacement);
        if !edit.replacement.is_empty() && !edit.replacement.ends_with('\n') {
            out.push('\n');
        }
        cursor = edit.end;
    }

    out.push_str(slice_span(source_text, cursor, source_text.len()));
    Ok(out)
}

fn slice_span(source_text: &str, start: usize, end: usize) -> &str {
    debug_assert!(source_text.is_char_boundary(start));
    debug_assert!(source_text.is_char_boundary(end));
    &source_text[start..end]
}

fn parse_program<'a>(
    allocator: &'a Allocator,
    source_path: &Path,
    source_text: &'a str,
    source_type: oxc::span::SourceType,
    parse_regular_expression: bool,
) -> Result<Program<'a>, UiJsError> {
    let parser_return = Parser::new(allocator, source_text, source_type)
        .with_options(ParseOptions {
            parse_regular_expression,
            ..ParseOptions::default()
        })
        .parse();

    if !parser_return.errors.is_empty() {
        let diagnostics = parser_return
            .errors
            .into_iter()
            .map(JsDiagnostic::from)
            .collect();
        return Err(UiJsError::Parse {
            file: source_path.to_path_buf(),
            diagnostics,
        });
    }

    Ok(parser_return.program)
}

fn build_semantic<'a>(
    source_path: &Path,
    program: &'a Program<'a>,
) -> Result<Semantic<'a>, UiJsError> {
    let semantic_return = SemanticBuilder::new()
        .with_check_syntax_error(true)
        .with_excess_capacity(0.10)
        .build(program);

    if !semantic_return.errors.is_empty() {
        let diagnostics = semantic_return
            .errors
            .into_iter()
            .map(JsDiagnostic::from)
            .collect();
        return Err(UiJsError::Semantic {
            file: source_path.to_path_buf(),
            diagnostics,
        });
    }

    Ok(semantic_return.semantic)
}

fn analyze_program<'a>(
    allocator: &'a Allocator,
    source_path: &Path,
    source_text: &str,
    program: &mut Program<'a>,
    scoping: Scoping,
) -> AnalysisReport {
    let mut analyzer = Analyzer::default();
    let _ = traverse_mut(&mut analyzer, allocator, program, scoping, ());
    analyzer.finish(source_path, source_text)
}

fn enforce_csp_mode(
    source_path: &Path,
    report: &AnalysisReport,
    csp_mode: CspMode,
) -> Result<(), UiJsError> {
    if matches!(csp_mode, CspMode::Strict) && report.has_strict_csp_violations() {
        let violations = report
            .strict_csp_violations()
            .into_iter()
            .map(str::to_owned)
            .collect();
        return Err(UiJsError::CspViolation {
            file: source_path.to_path_buf(),
            violations,
        });
    }

    Ok(())
}

fn emit_program<'a>(
    allocator: &'a Allocator,
    program: &mut Program<'a>,
    report: &AnalysisReport,
    options: &ObfuscatorOptions,
) -> (String, bool) {
    Compressor::new(allocator).build(program, CompressOptions::default());

    let should_mangle =
        options.should_mangle() && (options.allow_risky_mangle || !report.should_avoid_mangling());

    let mangler_return = if should_mangle {
        Some(
            Mangler::new()
                .with_options(MangleOptions::default())
                .build(program),
        )
    } else {
        None
    };
    let (mangled_scoping, private_member_mappings) = if let Some(return_value) = mangler_return {
        (
            Some(return_value.scoping),
            Some(return_value.class_private_mappings),
        )
    } else {
        (None, None)
    };

    let mut codegen_options = CodegenOptions::minify();
    codegen_options.single_quote = options.single_quote;

    let generated = Codegen::new()
        .with_options(codegen_options)
        .with_scoping(mangled_scoping)
        .with_private_member_mappings(private_member_mappings)
        .build(program);

    (generated.code, should_mangle)
}
