#![cfg(test)]

use crate::analysis;
use crate::analysis::StackParam;
use crate::analysis::TypeInfo;
use crate::analysis::{stack_usage, AnalysisError, AnalysisResult, StackUsage};
use crate::rt::parser::parse;
use crate::rt::val::SymbolTable;

fn analyze(src: &str) -> AnalysisResult {
    let mut t = SymbolTable::new();
    let p = parse(src, &mut t).expect("Invalid test program");
    analysis::analyze(&t, &p)
}

#[test]
fn stack_basics() {
    let cases = [
        ("", 0),
        ("1", 1),
        ("1 2 3", 3),
        ("drop 1 2", 1),
        ("swap 1 2 3", 3),
        ("dup 1", 2),
    ];

    for (src, out) in cases {
        matches_usage(src, StackUsage::from_in_out_sizes(0, out));
    }

    assert!(matches!(
        analyze("drop"),
        Err(AnalysisError::StackMismatch(_, x, y)) if x.is_empty() && y.in_len() == 1
    ));
    assert!(matches!(
        analyze("swap 1"),
        Err(AnalysisError::StackMismatch(_, x, y)) if x.len() == 1 && y.in_len() == 2
    ));
}

fn matches_usage(src: &str, exp: StackUsage) {
    let r = analyze(src).unwrap();
    assert!(
        r.matches_exact(&exp),
        "Stacks didn't match for `{}`:\n\tGot: {:?}\n\tExpected: {:?}",
        src,
        r,
        exp
    );
}

#[test]
fn typed() {
    matches_usage("+ 1 2", stack_usage!(() -> (int)));

    matches_usage("+ swap 1 2", stack_usage!(() -> (int)));

    assert!(matches!(
        analyze("+ 1 {}"),
        Err(AnalysisError::StackMismatch(_, x, y)) if y == stack_usage!((int int) -> (int))
    ));

    assert!(matches!(
        analyze("unquote 1"),
        Err(AnalysisError::StackMismatch(_, x, y)) if y == stack_usage!((anycode) -> (eval(0)))
    ));

    matches_usage("unquote {+ 1} 1", stack_usage!(() -> (int)));
    matches_usage("unquote swap 1 {+ 1}", stack_usage!(() -> (int)));
    matches_usage("unquote {swap} :wow 2", stack_usage!(() -> (int kw)))
}
