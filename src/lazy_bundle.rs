use std::collections::hash_map::DefaultHasher;
use std::ffi::OsString;
use std::fs;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use thiserror::Error;

use crate::{CspMode, JsError, SourceFlavor, minify};

#[derive(Debug, Clone)]
struct Binding {
    input_path: PathBuf,
    output_path: PathBuf,
}

impl Binding {
    fn hash_path(&self) -> PathBuf {
        let file_name = self
            .output_path
            .file_name()
            .map_or_else(|| OsString::from("bundle.js"), OsString::from);
        let mut hash_name = file_name;
        hash_name.push(".lazy-bundle.hash");
        self.output_path.with_file_name(hash_name)
    }
}

/// Controls how a lazy bundle should be post-processed after `bun` emits JS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LazyBundleMinifyLevel {
    /// Only bundle/transpile the input with `bun`; do not run the Rust minifier.
    None,
    /// Bundle with `bun`, then minify the emitted JavaScript with `rs-js`.
    #[default]
    Syntax,
    /// Alias for the current Rust minify pipeline.
    ///
    /// `rs-js` does not currently expose a whitespace-only mode, so this maps to
    /// the same post-processing pass as `Syntax`.
    Whitespace,
    /// Bundle with `bun`, then obfuscate/minify the emitted JavaScript with `rs-js`.
    Identifiers,
}

impl LazyBundleMinifyLevel {
    fn post_process(
        self,
        output_path: &Path,
        source_text: &str,
        csp_mode: CspMode,
    ) -> Result<String, LazyBundleError> {
        let source_name = output_path.to_string_lossy();
        let builder = minify(source_name.as_ref(), source_text)
            .javascript()
            .csp_mode(csp_mode);

        let result = match self {
            Self::None => return Ok(source_text.to_owned()),
            Self::Whitespace | Self::Syntax => builder.run(),
            Self::Identifiers => builder.obfuscate().run(),
        };

        result
            .map(|processed| processed.code)
            .map_err(|source| LazyBundleError::Minify {
                output: output_path.to_path_buf(),
                source,
            })
    }
}

/// Error returned by the lazy bundling helper.
#[derive(Debug, Error)]
pub enum LazyBundleError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported lazy bundle input type: {}", .0.display())]
    UnsupportedSourceType(PathBuf),
    #[error("bun is required to bundle {}: {source}", input.display())]
    BunUnavailable {
        input: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("bun build failed for {} with status {status}", input.display())]
    BunFailed { input: PathBuf, status: ExitStatus },
    #[error("rs-js post-processing failed for {}: {source}", output.display())]
    Minify {
        output: PathBuf,
        #[source]
        source: JsError,
    },
}

/// Builder for lazily bundling TypeScript/JavaScript assets into JavaScript.
///
/// This helper is intentionally small:
/// - `bun` handles module bundling / TS transpilation
/// - `rs-js` optionally minifies the emitted JavaScript
/// - a content hash prevents unnecessary rebuilds
#[derive(Debug)]
pub struct LazyBundleBuilder {
    bindings: Vec<Binding>,
    minify_level: LazyBundleMinifyLevel,
    watching: Vec<PathBuf>,
    csp_mode: CspMode,
}

impl Default for LazyBundleBuilder {
    fn default() -> Self {
        Self {
            bindings: Vec::new(),
            minify_level: LazyBundleMinifyLevel::default(),
            watching: Vec::new(),
            csp_mode: CspMode::Off,
        }
    }
}

impl LazyBundleBuilder {
    /// Creates a new lazy bundle builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an input/output bundle binding.
    #[must_use]
    pub fn with_binding(
        mut self,
        input_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
    ) -> Self {
        self.bindings.push(Binding {
            input_path: input_path.as_ref().to_path_buf(),
            output_path: output_path.as_ref().to_path_buf(),
        });
        self
    }

    /// Sets the post-processing level for emitted JavaScript.
    #[must_use]
    pub fn with_minify_level(mut self, minify_level: LazyBundleMinifyLevel) -> Self {
        self.minify_level = minify_level;
        self
    }

    /// Adds an extra path or directory to watch for hash invalidation.
    #[must_use]
    pub fn with_watching(mut self, path: impl AsRef<Path>) -> Self {
        self.watching.push(path.as_ref().to_path_buf());
        self
    }

    /// Sets the CSP policy used by the optional `rs-js` post-processing step.
    #[must_use]
    pub fn with_csp_mode(mut self, csp_mode: CspMode) -> Self {
        self.csp_mode = csp_mode;
        self
    }

    /// Uses strict CSP validation during the optional `rs-js` post-processing step.
    #[must_use]
    pub fn strict_csp(self) -> Self {
        self.with_csp_mode(CspMode::Strict)
    }

    /// Uses compatible CSP validation during the optional `rs-js` post-processing step.
    #[must_use]
    pub fn compatible_csp(self) -> Self {
        self.with_csp_mode(CspMode::Compatible)
    }

    /// Disables CSP validation during the optional `rs-js` post-processing step.
    #[must_use]
    pub fn without_csp(self) -> Self {
        self.with_csp_mode(CspMode::Off)
    }

    /// Runs the configured lazy bundle pipeline.
    ///
    /// # Errors
    ///
    /// Returns an error if file I/O fails, `bun` is unavailable or fails, or
    /// the optional `rs-js` post-processing step rejects the emitted JavaScript.
    pub fn run(&self) -> Result<(), LazyBundleError> {
        let watched_files = self.collect_watched_files()?;
        let watched_hash = hash_files(&watched_files)?;

        for path in &watched_files {
            println!("cargo:rerun-if-changed={}", path.display());
        }

        for binding in &self.bindings {
            if binding_is_fresh(binding, watched_hash)? {
                continue;
            }

            validate_input_path(&binding.input_path)?;
            bundle_with_bun(&binding.input_path, &binding.output_path)?;

            let bundled = fs::read_to_string(&binding.output_path)?;
            let output =
                self.minify_level
                    .post_process(&binding.output_path, &bundled, self.csp_mode)?;
            fs::write(&binding.output_path, output)?;
            fs::write(binding.hash_path(), watched_hash.to_string())?;
        }

        Ok(())
    }

    fn collect_watched_files(&self) -> Result<Vec<PathBuf>, LazyBundleError> {
        let mut files = self
            .bindings
            .iter()
            .map(|binding| binding.input_path.clone())
            .collect::<Vec<_>>();

        for path in &self.watching {
            if path.is_dir() {
                for entry in fs::read_dir(path)? {
                    let path = entry?.path();
                    if is_watchable_source(&path) {
                        files.push(path);
                    }
                }
            } else {
                files.push(path.clone());
            }
        }

        files.sort();
        files.dedup();
        Ok(files)
    }
}

fn binding_is_fresh(binding: &Binding, watched_hash: u64) -> Result<bool, LazyBundleError> {
    let expected = match fs::read_to_string(binding.hash_path()) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    Ok(expected.trim() == watched_hash.to_string())
}

fn validate_input_path(input_path: &Path) -> Result<(), LazyBundleError> {
    if SourceFlavor::from_path(input_path).is_some() {
        Ok(())
    } else {
        Err(LazyBundleError::UnsupportedSourceType(
            input_path.to_path_buf(),
        ))
    }
}

fn is_watchable_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs")
    )
}

fn hash_files(files: &[PathBuf]) -> Result<u64, LazyBundleError> {
    let mut hash = DefaultHasher::new();

    for file in files {
        hash.write(file.to_string_lossy().as_bytes());
        let contents = match fs::read_to_string(file) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        for line in contents.lines() {
            hash.write(line.as_bytes());
            hash.write_u8(b'\n');
        }
    }

    Ok(hash.finish())
}

fn bundle_with_bun(input_path: &Path, output_path: &Path) -> Result<(), LazyBundleError> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let status = Command::new("bun")
        .arg("build")
        .arg(input_path)
        .arg("--outfile")
        .arg(output_path)
        .status()
        .map_err(|source| LazyBundleError::BunUnavailable {
            input: input_path.to_path_buf(),
            source,
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(LazyBundleError::BunFailed {
            input: input_path.to_path_buf(),
            status,
        })
    }
}
