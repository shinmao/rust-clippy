use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{intravisit::FnKind, Body, FnDecl};
use rustc_lint::{LateContext, LintContext};
use rustc_span::Span;

use super::FOO_FUNCTIONS;

// TODO: Adjust the parameters as necessary
pub(super) fn check_fn(cx: &LateContext<'_>, kind: FnKind<'_>, decl: &FnDecl<'_>, body: &Body<'_>, span: Span) {
    if is_foo_fn(kind) {
        span_lint_and_help(
            cx,
            FOO_FUNCTIONS,
            span,
            "function name `foo`",
            None,
            "You should use more meaningful name",
        );
    }
}

fn is_foo_fn(fn_kind: FnKind<'_>) -> bool {
    match fn_kind {
        FnKind::ItemFn(ident, _, header) => {
            // check if `fn` name is `foo`
            ident.name.as_str() == "foo"
        },
        FnKind::Method(ident, _) => ident.name.as_str() == "foo",
        FnKind::Closure => false,
    }
}
