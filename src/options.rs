use std::path::Path;

use oxc::span::SourceType;

use crate::error::UiJsError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Analyze,
    Minify,
    Obfuscate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFlavor {
    Js,
    Jsx,
    Ts,
    Tsx,
    Mjs,
    Cjs,
}

impl SourceFlavor {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Js => "js",
            Self::Jsx => "jsx",
            Self::Ts => "ts",
            Self::Tsx => "tsx",
            Self::Mjs => "mjs",
            Self::Cjs => "cjs",
        }
    }

    /// Converts the source flavor into an Oxc source type.
    ///
    /// # Panics
    ///
    /// Panics only if one of the fixed internal extension probes stops being
    /// accepted by `oxc::span::SourceType`.
    #[must_use]
    pub fn to_source_type(self) -> SourceType {
        match self {
            Self::Js => SourceType::from_path("input.js").expect("static js path must be valid"),
            Self::Jsx => SourceType::from_path("input.jsx").expect("static jsx path must be valid"),
            Self::Ts => SourceType::from_path("input.ts").expect("static ts path must be valid"),
            Self::Tsx => SourceType::from_path("input.tsx").expect("static tsx path must be valid"),
            Self::Mjs => SourceType::from_path("input.mjs").expect("static mjs path must be valid"),
            Self::Cjs => SourceType::from_path("input.cjs").expect("static cjs path must be valid"),
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "js" => Some(Self::Js),
            "jsx" => Some(Self::Jsx),
            "ts" => Some(Self::Ts),
            "tsx" => Some(Self::Tsx),
            "mjs" => Some(Self::Mjs),
            "cjs" => Some(Self::Cjs),
            _ => None,
        }
    }

    #[must_use]
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension().and_then(|value| value.to_str()) {
            Some("js") => Some(Self::Js),
            Some("jsx") => Some(Self::Jsx),
            Some("ts") => Some(Self::Ts),
            Some("tsx") => Some(Self::Tsx),
            Some("mjs") => Some(Self::Mjs),
            Some("cjs") => Some(Self::Cjs),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CspMode {
    Off,
    Compatible,
    Strict,
}

impl CspMode {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "off" => Some(Self::Off),
            "compatible" | "compat" => Some(Self::Compatible),
            "strict" => Some(Self::Strict),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObfuscatorOptions {
    pub mode: Mode,
    pub source_flavor: Option<SourceFlavor>,
    pub single_quote: bool,
    pub parse_regular_expression: bool,
    pub allow_risky_mangle: bool,
    pub csp_mode: CspMode,
}

impl Default for ObfuscatorOptions {
    fn default() -> Self {
        Self {
            mode: Mode::Obfuscate,
            source_flavor: None,
            single_quote: true,
            parse_regular_expression: true,
            allow_risky_mangle: false,
            csp_mode: CspMode::Compatible,
        }
    }
}

impl ObfuscatorOptions {
    #[must_use]
    pub fn analyze() -> Self {
        Self {
            mode: Mode::Analyze,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn minify() -> Self {
        Self {
            mode: Mode::Minify,
            csp_mode: CspMode::Strict,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn obfuscate() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_source_flavor(mut self, flavor: SourceFlavor) -> Self {
        self.source_flavor = Some(flavor);
        self
    }

    #[must_use]
    pub fn with_csp_mode(mut self, mode: CspMode) -> Self {
        self.csp_mode = mode;
        self
    }

    #[must_use]
    pub fn with_single_quote(mut self, single_quote: bool) -> Self {
        self.single_quote = single_quote;
        self
    }

    #[must_use]
    pub fn with_parse_regular_expression(mut self, parse_regular_expression: bool) -> Self {
        self.parse_regular_expression = parse_regular_expression;
        self
    }

    #[must_use]
    pub fn with_allow_risky_mangle(mut self, allow_risky_mangle: bool) -> Self {
        self.allow_risky_mangle = allow_risky_mangle;
        self
    }

    /// Resolves the effective source flavor for an input path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file extension is unsupported and no explicit
    /// source flavor override was configured.
    pub fn source_flavor_for_path(&self, path: &Path) -> Result<SourceFlavor, UiJsError> {
        if let Some(flavor) = self.source_flavor {
            return Ok(flavor);
        }

        SourceFlavor::from_path(path)
            .ok_or_else(|| UiJsError::UnsupportedSourceType(path.to_path_buf()))
    }

    /// Resolves the effective source type for an input path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file extension is unsupported and no explicit
    /// source flavor override was configured.
    pub fn source_type_for_path(&self, path: &Path) -> Result<SourceType, UiJsError> {
        self.source_flavor_for_path(path)
            .map(SourceFlavor::to_source_type)
    }

    #[must_use]
    pub fn should_minify(&self) -> bool {
        matches!(self.mode, Mode::Minify | Mode::Obfuscate)
    }

    #[must_use]
    pub fn should_mangle(&self) -> bool {
        matches!(self.mode, Mode::Obfuscate)
    }
}
