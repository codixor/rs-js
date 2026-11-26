#![doc = include_str!("../README.md")]

pub mod analysis;
pub mod compat {
    pub use crate::pipeline::{
        analyze_javascript, analyze_source, minify_embeddable, minify_embeddable_javascript,
        minify_javascript, minify_source, normalize_embeddable_javascript, obfuscate_javascript,
        obfuscate_source,
    };
}
pub mod error;
pub mod lazy_bundle;
pub mod options;
pub mod pipeline;

pub use analysis::{AnalysisReport, RiskFlag};
pub use error::{DiagnosticLabel, DiagnosticSeverity, JsDiagnostic, JsError, UiJsError};
pub use options::{CspMode, Mode, ObfuscatorOptions, SourceFlavor};
pub use pipeline::{MinifyBuilder, ProcessedOutput, minify, normalize_embeddable, process_source};
