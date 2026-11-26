use std::{
    collections::BTreeSet,
    hash::{Hash, Hasher},
    path::Path,
};

use oxc::ast::ast::{
    BindingIdentifier, CallExpression, Class, DebuggerStatement, ExportAllDeclaration,
    ExportDefaultDeclaration, ExportNamedDeclaration, Function, IdentifierReference,
    ImportDeclaration, ImportExpression, JSXElement, JSXFragment, NumericLiteral,
    PrivateIdentifier, StringLiteral, WithStatement,
};
use oxc_traverse::{Traverse, TraverseCtx};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RiskFlag {
    DirectEval,
    IndirectEval,
    FunctionConstructor,
    StringTimerCallback,
    DynamicImport,
    WithStatement,
    DebuggerStatement,
    JsxPresent,
    HasExports,
}

#[derive(Debug, Clone)]
pub struct AnalysisReport {
    pub file_name: String,
    pub fingerprint: u64,
    pub bindings: u64,
    pub identifier_references: u64,
    pub functions: u64,
    pub classes: u64,
    pub string_literals: u64,
    pub numeric_literals: u64,
    pub private_identifiers: u64,
    pub imports: u64,
    pub exports: u64,
    pub dynamic_imports: u64,
    pub direct_eval_calls: u64,
    pub indirect_eval_calls: u64,
    pub function_constructor_uses: u64,
    pub string_timer_callbacks: u64,
    pub debugger_statements: u64,
    pub with_statements: u64,
    pub jsx_nodes: u64,
    pub risk_flags: Vec<RiskFlag>,
}

impl AnalysisReport {
    #[must_use]
    pub fn should_avoid_mangling(&self) -> bool {
        self.risk_flags.iter().any(|flag| {
            matches!(
                flag,
                RiskFlag::DirectEval
                    | RiskFlag::IndirectEval
                    | RiskFlag::FunctionConstructor
                    | RiskFlag::StringTimerCallback
                    | RiskFlag::WithStatement
            )
        })
    }

    #[must_use]
    pub fn has_strict_csp_violations(&self) -> bool {
        self.risk_flags.iter().any(|flag| {
            matches!(
                flag,
                RiskFlag::DirectEval
                    | RiskFlag::IndirectEval
                    | RiskFlag::FunctionConstructor
                    | RiskFlag::StringTimerCallback
            )
        })
    }

    #[must_use]
    pub fn strict_csp_violations(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        for flag in &self.risk_flags {
            match flag {
                RiskFlag::DirectEval => {
                    out.push("eval() is blocked by strict CSP unless 'unsafe-eval' is allowed");
                }
                RiskFlag::IndirectEval => {
                    out.push("indirect eval-style calls are blocked by strict CSP unless 'unsafe-eval' is allowed");
                }
                RiskFlag::FunctionConstructor => {
                    out.push("Function-style string compilation was detected and is blocked by strict CSP unless 'unsafe-eval' is allowed");
                }
                RiskFlag::StringTimerCallback => {
                    out.push("string callbacks in setTimeout/setInterval/setImmediate are blocked by strict CSP unless 'unsafe-eval' is allowed");
                }
                _ => {}
            }
        }
        out
    }

    #[must_use]
    pub fn warning_messages(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        for flag in &self.risk_flags {
            match flag {
                RiskFlag::DirectEval => out.push("direct eval detected; strict CSP will reject this unless unsafe-eval is enabled"),
                RiskFlag::IndirectEval => out.push("indirect eval pattern detected; strict CSP will reject this unless unsafe-eval is enabled"),
                RiskFlag::FunctionConstructor => out.push("Function-style constructor detected; strict CSP will reject this unless unsafe-eval is enabled"),
                RiskFlag::StringTimerCallback => out.push("string timer callback detected; strict CSP will reject this unless unsafe-eval is enabled"),
                RiskFlag::DynamicImport => out.push("dynamic import detected; keep deployment path checks in CI"),
                RiskFlag::WithStatement => out.push("with statement detected; identifier mangling is conservatively unsafe"),
                RiskFlag::DebuggerStatement => out.push("debugger statement detected in input"),
                RiskFlag::JsxPresent => out.push("JSX detected; avoid aggressive top-level mangling unless your runtime is verified"),
                RiskFlag::HasExports => out.push("exports detected; public surface names should be validated in integration tests"),
            }
        }
        out
    }
}

#[derive(Debug, Default)]
pub struct Analyzer {
    bindings: u64,
    identifier_references: u64,
    functions: u64,
    classes: u64,
    string_literals: u64,
    numeric_literals: u64,
    private_identifiers: u64,
    imports: u64,
    exports: u64,
    dynamic_imports: u64,
    direct_eval_calls: u64,
    indirect_eval_calls: u64,
    function_constructor_uses: u64,
    string_timer_callbacks: u64,
    debugger_statements: u64,
    with_statements: u64,
    jsx_nodes: u64,
    risk_flags: BTreeSet<RiskFlag>,
}

impl Analyzer {
    #[must_use]
    pub fn finish(mut self, file: &Path, source_text: &str) -> AnalysisReport {
        self.scan_source_text(source_text);

        let fingerprint = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            self.bindings.hash(&mut hasher);
            self.identifier_references.hash(&mut hasher);
            self.functions.hash(&mut hasher);
            self.classes.hash(&mut hasher);
            self.string_literals.hash(&mut hasher);
            self.numeric_literals.hash(&mut hasher);
            self.private_identifiers.hash(&mut hasher);
            self.imports.hash(&mut hasher);
            self.exports.hash(&mut hasher);
            self.dynamic_imports.hash(&mut hasher);
            self.direct_eval_calls.hash(&mut hasher);
            self.indirect_eval_calls.hash(&mut hasher);
            self.function_constructor_uses.hash(&mut hasher);
            self.string_timer_callbacks.hash(&mut hasher);
            self.debugger_statements.hash(&mut hasher);
            self.with_statements.hash(&mut hasher);
            self.jsx_nodes.hash(&mut hasher);
            self.risk_flags.hash(&mut hasher);
            hasher.finish()
        };

        AnalysisReport {
            file_name: file.display().to_string(),
            fingerprint,
            bindings: self.bindings,
            identifier_references: self.identifier_references,
            functions: self.functions,
            classes: self.classes,
            string_literals: self.string_literals,
            numeric_literals: self.numeric_literals,
            private_identifiers: self.private_identifiers,
            imports: self.imports,
            exports: self.exports,
            dynamic_imports: self.dynamic_imports,
            direct_eval_calls: self.direct_eval_calls,
            indirect_eval_calls: self.indirect_eval_calls,
            function_constructor_uses: self.function_constructor_uses,
            string_timer_callbacks: self.string_timer_callbacks,
            debugger_statements: self.debugger_statements,
            with_statements: self.with_statements,
            jsx_nodes: self.jsx_nodes,
            risk_flags: self.risk_flags.into_iter().collect(),
        }
    }

    fn scan_source_text(&mut self, source_text: &str) {
        let indirect_eval_hits = count_any(
            source_text,
            &[
                "(0, eval)(",
                "(0,eval)(",
                "window.eval(",
                "globalThis.eval(",
                "self.eval(",
                "global.eval(",
                "eval.call(",
                "eval.apply(",
            ],
        );
        if indirect_eval_hits > 0 {
            self.indirect_eval_calls += indirect_eval_hits;
            self.risk_flags.insert(RiskFlag::IndirectEval);
        }

        let function_hits = count_any(
            source_text,
            &[
                "new Function(",
                "new Function ",
                "Function(",
                "AsyncFunction(",
                "GeneratorFunction(",
                "AsyncGeneratorFunction(",
            ],
        );
        if function_hits > 0 {
            self.function_constructor_uses += function_hits;
            self.risk_flags.insert(RiskFlag::FunctionConstructor);
        }

        let string_timer_hits = count_any(
            source_text,
            &[
                "setTimeout(\"",
                "setTimeout('",
                "setInterval(\"",
                "setInterval('",
                "setImmediate(\"",
                "setImmediate('",
            ],
        );
        if string_timer_hits > 0 {
            self.string_timer_callbacks += string_timer_hits;
            self.risk_flags.insert(RiskFlag::StringTimerCallback);
        }
    }
}

fn count_any(haystack: &str, needles: &[&str]) -> u64 {
    needles
        .iter()
        .map(|needle| haystack.match_indices(needle).count() as u64)
        .sum()
}

impl<'a> Traverse<'a, ()> for Analyzer {
    fn enter_binding_identifier(
        &mut self,
        _node: &mut BindingIdentifier<'a>,
        _ctx: &mut TraverseCtx<'a, ()>,
    ) {
        self.bindings += 1;
    }

    fn enter_identifier_reference(
        &mut self,
        _node: &mut IdentifierReference<'a>,
        _ctx: &mut TraverseCtx<'a, ()>,
    ) {
        self.identifier_references += 1;
    }

    fn enter_function(&mut self, _node: &mut Function<'a>, _ctx: &mut TraverseCtx<'a, ()>) {
        self.functions += 1;
    }

    fn enter_class(&mut self, _node: &mut Class<'a>, _ctx: &mut TraverseCtx<'a, ()>) {
        self.classes += 1;
    }

    fn enter_string_literal(
        &mut self,
        _node: &mut StringLiteral<'a>,
        _ctx: &mut TraverseCtx<'a, ()>,
    ) {
        self.string_literals += 1;
    }

    fn enter_numeric_literal(
        &mut self,
        _node: &mut NumericLiteral<'a>,
        _ctx: &mut TraverseCtx<'a, ()>,
    ) {
        self.numeric_literals += 1;
    }

    fn enter_private_identifier(
        &mut self,
        _node: &mut PrivateIdentifier<'a>,
        _ctx: &mut TraverseCtx<'a, ()>,
    ) {
        self.private_identifiers += 1;
    }

    fn enter_import_declaration(
        &mut self,
        _node: &mut ImportDeclaration<'a>,
        _ctx: &mut TraverseCtx<'a, ()>,
    ) {
        self.imports += 1;
    }

    fn enter_import_expression(
        &mut self,
        _node: &mut ImportExpression<'a>,
        _ctx: &mut TraverseCtx<'a, ()>,
    ) {
        self.dynamic_imports += 1;
        self.risk_flags.insert(RiskFlag::DynamicImport);
    }

    fn enter_export_named_declaration(
        &mut self,
        _node: &mut ExportNamedDeclaration<'a>,
        _ctx: &mut TraverseCtx<'a, ()>,
    ) {
        self.exports += 1;
        self.risk_flags.insert(RiskFlag::HasExports);
    }

    fn enter_export_default_declaration(
        &mut self,
        _node: &mut ExportDefaultDeclaration<'a>,
        _ctx: &mut TraverseCtx<'a, ()>,
    ) {
        self.exports += 1;
        self.risk_flags.insert(RiskFlag::HasExports);
    }

    fn enter_export_all_declaration(
        &mut self,
        _node: &mut ExportAllDeclaration<'a>,
        _ctx: &mut TraverseCtx<'a, ()>,
    ) {
        self.exports += 1;
        self.risk_flags.insert(RiskFlag::HasExports);
    }

    fn enter_jsx_element(&mut self, _node: &mut JSXElement<'a>, _ctx: &mut TraverseCtx<'a, ()>) {
        self.jsx_nodes += 1;
        self.risk_flags.insert(RiskFlag::JsxPresent);
    }

    fn enter_jsx_fragment(&mut self, _node: &mut JSXFragment<'a>, _ctx: &mut TraverseCtx<'a, ()>) {
        self.jsx_nodes += 1;
        self.risk_flags.insert(RiskFlag::JsxPresent);
    }

    fn enter_call_expression(
        &mut self,
        node: &mut CallExpression<'a>,
        _ctx: &mut TraverseCtx<'a, ()>,
    ) {
        if let oxc::ast::ast::Expression::Identifier(identifier) = &node.callee
            && identifier.name.as_str() == "eval"
        {
            self.direct_eval_calls += 1;
            self.risk_flags.insert(RiskFlag::DirectEval);
        }
    }

    fn enter_debugger_statement(
        &mut self,
        _node: &mut DebuggerStatement,
        _ctx: &mut TraverseCtx<'a, ()>,
    ) {
        self.debugger_statements += 1;
        self.risk_flags.insert(RiskFlag::DebuggerStatement);
    }

    fn enter_with_statement(
        &mut self,
        _node: &mut WithStatement<'a>,
        _ctx: &mut TraverseCtx<'a, ()>,
    ) {
        self.with_statements += 1;
        self.risk_flags.insert(RiskFlag::WithStatement);
    }
}
