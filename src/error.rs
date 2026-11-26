use std::{io, path::PathBuf};

use oxc::diagnostics::{OxcDiagnostic, Severity};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Advice,
    Warning,
    Error,
}

impl DiagnosticSeverity {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Advice => "advice",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

impl From<Severity> for DiagnosticSeverity {
    fn from(value: Severity) -> Self {
        match value {
            Severity::Advice => Self::Advice,
            Severity::Warning => Self::Warning,
            Severity::Error => Self::Error,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLabel {
    pub label: Option<String>,
    pub offset: usize,
    pub length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsDiagnostic {
    pub message: String,
    pub severity: DiagnosticSeverity,
    pub code: Option<String>,
    pub help: Option<String>,
    pub labels: Vec<DiagnosticLabel>,
}

impl JsDiagnostic {
    #[must_use]
    pub fn summary_line(&self) -> String {
        let mut out = format!("{}: {}", self.severity.as_str(), self.message);
        if let Some(code) = &self.code {
            out.push_str(" [");
            out.push_str(code);
            out.push(']');
        }
        if let Some(label) = self.labels.first() {
            out.push_str(" @ ");
            out.push_str(&label.offset.to_string());
            out.push_str("..");
            out.push_str(&(label.offset + label.length).to_string());
        }
        out
    }
}

impl std::fmt::Display for JsDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.summary_line())
    }
}

impl From<OxcDiagnostic> for JsDiagnostic {
    fn from(diagnostic: OxcDiagnostic) -> Self {
        let severity = DiagnosticSeverity::from(diagnostic.severity);
        let code = diagnostic
            .code
            .is_some()
            .then(|| diagnostic.code.to_string());
        let help = diagnostic.help.as_ref().map(ToString::to_string);
        let labels = diagnostic
            .labels
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|label| DiagnosticLabel {
                label: label.label().map(ToString::to_string),
                offset: label.offset(),
                length: label.len(),
            })
            .collect();

        Self {
            message: diagnostic.message.to_string(),
            severity,
            code,
            help,
            labels,
        }
    }
}

#[derive(Debug, Error)]
pub enum UiJsError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("could not infer source type from path: {}", .0.display())]
    UnsupportedSourceType(PathBuf),
    #[error("parse failed for {}{}", file.display(), format_lines(diagnostics))]
    Parse {
        file: PathBuf,
        diagnostics: Vec<JsDiagnostic>,
    },
    #[error("semantic analysis failed for {}{}", file.display(), format_lines(diagnostics))]
    Semantic {
        file: PathBuf,
        diagnostics: Vec<JsDiagnostic>,
    },
    #[error("strict CSP gate failed for {}{}", file.display(), format_lines(violations))]
    CspViolation {
        file: PathBuf,
        violations: Vec<String>,
    },
    #[error(
        "embeddable bundle normalization failed for {}{}",
        file.display(),
        format_lines(reasons)
    )]
    UnsupportedEmbeddableModuleSyntax { file: PathBuf, reasons: Vec<String> },
}

impl UiJsError {
    #[must_use]
    pub fn diagnostics(&self) -> &[JsDiagnostic] {
        match self {
            Self::Parse { diagnostics, .. } | Self::Semantic { diagnostics, .. } => diagnostics,
            _ => &[],
        }
    }
}

fn format_lines<T>(lines: &[T]) -> String
where
    T: DiagnosticLine,
{
    if lines.is_empty() {
        String::new()
    } else {
        let mut out = String::new();
        for line in lines {
            out.push_str("\n- ");
            out.push_str(&line.diagnostic_line());
        }
        out
    }
}

trait DiagnosticLine {
    fn diagnostic_line(&self) -> String;
}

impl DiagnosticLine for JsDiagnostic {
    fn diagnostic_line(&self) -> String {
        self.summary_line()
    }
}

impl DiagnosticLine for String {
    fn diagnostic_line(&self) -> String {
        self.clone()
    }
}

pub type JsError = UiJsError;
