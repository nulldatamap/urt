#![cfg(test)]

use crate::eval::eval;
use crate::parser::parse;
use crate::val::{Program, Val, Values};

fn evals(program: &'static str, stack: &'static str) {
    let p = parse(program).unwrap();
    let exp = parse(stack).unwrap();
    match eval(p) {
        Ok(got) => assert_eq!(got, exp, "evaluating: {}", program),
        Err(e) => {
            panic!(
                "Evaluation failed:\n{:?} | {:?}",
                Program(&e.program),
                Values(&e.stack)
            );
        }
    }
}

fn fails(program: &'static str, fail_tail: &'static str, stack: &'static str) {
    let p = parse(program).unwrap();
    let p0 = parse(fail_tail).unwrap();
    let mut exp0: Vec<Val> = parse(stack).unwrap().into();
    exp0.reverse();
    match eval(p) {
        Ok(got) => panic!(
            "Expected program to fail!\nProgram: {:?}\nResult: {:?}",
            program,
            Values(&Vec::from(got))
        ),
        Err(e) => {
            let mut matches = false;
            if e.program.len() >= p0.len() {
                matches = true;
                for (x0, x1) in e.program.iter().rev().zip(p0.iter().rev()) {
                    if x0 != x1 {
                        matches = false;
                        break;
                    }
                }
            }
            if matches {
                matches = e.stack == exp0;
            }
            assert!(
                matches,
                "Unexpected program fail state!\nExpected: {:?} | {:?}\nGot: {:?} | {:?}",
                Program(&p0),
                Values(&exp0),
                Program(&e.program),
                Values(&e.stack)
            );
        }
    }
}

#[test]
fn basics() {
    evals("1 2 3", "1 2 3");
    evals("{1 2 3}", "{1 2 3}");
    evals("-1", "-1");
    evals("{a b c}", "{a b c}");
    evals("{{} {{}} {}}", "{{} {{}} {}}");
}

#[test]
fn stack() {
    evals("drop 1", "");
    evals("drop 1 2", "2");
    fails("drop", "drop", "");

    evals("dup 1 2", "1 1 2");
    fails("dup", "dup", "");

    evals("swap 1 2", "2 1");
    fails("swap", "swap", "");
    fails("swap 1", "swap", "1");
}

#[test]
fn arithmetic() {
    evals("+ 1 2", "3");
    evals("- 4 2", "2");
    evals("* 2 2", "4");
    evals("/ 4 2", "2");
    evals("% 10 8", "2");

    fails("+ 1", "+", "1");
    fails("* {1} {2}", "*", "{1} {2}");
}

#[test]
fn comparison() {
    evals("= 1 1", "1");
    evals("= 1 2", "0");
    evals("= {1} {1}", "1");
    evals("= {1 2 3} {1 2 3}", "1");

    evals("<> 1 1", "0");
    evals("<> 1 2", "1");
    evals("<> {1} {1}", "0");
    evals("<> {1 2 3} {1 2 3}", "0");

    evals("< 1 2", "1");
    evals("< 2 0", "0");
    evals("> 2 1", "1");
    evals("> 0 1", "0");

    evals("<= 1 2", "1");
    evals("<= 2 2", "1");
    evals("<= 2 0", "0");
    evals(">= 2 1", "1");
    evals(">= 1 1", "1");
    evals(">= 0 1", "0");

    fails("> 1 {2}", ">", "1 {2}");
}

#[test]
fn logic() {
    evals("true", "1");
    evals("false", "0");

    evals("&& true true", "1");
    evals("&& false true", "0");
    evals("&& true false", "0");
    evals("&& false false", "0");
    evals("&& {1} {2}", "{2}");
    evals("&& {} {2}", "{}");
    evals("&& {2} {}", "{}");
    evals("&& 0 {}", "0");

    evals("|| true true", "1");
    evals("|| false true", "1");
    evals("|| true false", "1");
    evals("|| false false", "0");
    evals("|| {1} {2}", "{1}");
    evals("|| {} {2}", "{2}");
    evals("|| {2} {}", "{2}");
    evals("|| 0 {}", "{}");

    evals("not true", "0");
    evals("not false", "1");
    evals("not {1}", "0");
    evals("not {}", "1");
}

#[test]
fn quoting() {
    evals("quote 1", "{1}");
    evals("quote {}", "{{}}");

    evals("unquote {1}", "1");
    evals("unquote {+ 1 2}", "3");

    fails("unquote 1", "unquote", "1");
}

#[test]
fn control_flow() {
    evals("choose {1} {2} true", "1");
    evals("choose {1} {2} false", "2");
    evals("choose {+ 1 2} {2} true", "3");

    fails("choose 9 2 true", "choose", "9 2 1");
}

#[test]
fn scope() {
    evals("locals {x y} {+ x y} 1 2", "3");
    evals("locals {} {1} 2", "1 2");
    evals(
        "locals {x y} { + + x y locals {x} { + x y } 99 } 1 2",
        "104",
    );
    evals("locals {x} {x} 1 locals {x} {x} 2", "1 2");
    evals("locals {x} {locals {x} {locals {x} {x} 3} 2} 1", "3");
    fails("locals {x y} {x} 1", "locals", "{x y} {x} 1");
    fails("locals {1} {x} 1", "locals", " {1} {x} 1");
    fails("x locals {x} {x} 1", "x", "1");

    evals("define {} {1}", "1");
    evals("define {x {1} y {2}} {+ x y}", "3");
    evals("define {x {1}} { + x define {x {2}} { x } }", "3");
    evals("define {x {1}} {x} define {x {2}} {x}", "1 2");
    fails("define {x 1} {}", "define", " {x 1} {}");
    fails("define {x} {}", "define", " {x} {}");
    fails("x define {x {1}} {}", "x", "");
}

#[test]
fn recursion() {
    // Naive:
    evals(
        "define { fib { locals { x } { choose { + fib - x 1 fib - x 2 } {1} > x 2 } } }\
    { fib 6 }\
    ",
        "8",
    );
    // Iterative:
    evals(
        "
    define { fib { define { go { locals { a b n }
                                        { choose { a }
                                                 { go b
                                                      + a b
                                                      - n 1}
                                                      = n 0 } } }
                          { go 0 1 } } }
           { fib 6 }",
        "8",
    );
}

#[test]
fn list() {
    evals("length {}", "0");
    evals("length {1 2 3}", "3");
    fails("length 1", "length", "1");

    evals("append {} {}", "{}");
    evals("append {1} {}", "{1}");
    evals("append {} {2}", "{2}");
    evals("append {1} {2}", "{1 2}");
    evals("append dup {1 2 3}", "{1 2 3 1 2 3}");
    fails("append 1 {}", "append", "1 {}");
    fails("append {} 1", "append", "{} 1");

    evals("push-back 4 {1 2 3}", "{1 2 3 4}");
    evals("push-back 1 {}", "{1}");
    evals("push-back {1} {}", "{{1}}");
    fails("push-back 1 1", "push-back", "1 1");

    evals("push-front 0 {1 2 3}", "{0 1 2 3}");
    evals("push-front 1 {}", "{1}");
    evals("push-front {1} {}", "{{1}}");
    fails("push-front 1 1", "push-front", "1 1");

    evals("head {1 2 3}", "1");
    evals("head {1}", "1");
    fails("head 1", "head", "1");
    fails("head {}", "head", "{}");

    evals("tail {1 2 3}", "{2 3}");
    evals("tail {3}", "{}");
    fails("tail 1", "tail", "1");
    fails("tail {}", "tail", "{}");

    evals("init {1 2 3}", "{1 2}");
    evals("init {1}", "{}");
    fails("init 1", "init", "1");
    fails("init {}", "init", "{}");

    evals("last {1 2 3}", "3");
    evals("last {3}", "3");
    fails("last {}", "last", "{}");

    evals("nth 0 {1 2 3}", "1");
    evals("nth 2 {1 2 3}", "3");
    evals("nth -1 {1 2 3}", "3");
    evals("nth -3 {1 2 3}", "1");
    fails("nth 0 {}", "nth", "0 {}");
    fails("nth -1 {}", "nth", "-1 {}");
    fails("nth 1 {1}", "nth", "1 {1}");
    fails("nth 0 0", "nth", "0 0");
    fails("nth {1 2 3} 0", "nth", "{1 2 3} 0");

    evals("set-nth 0 99 {1 2 3}", "{99 2 3}");
    evals("set-nth 2 99 {1 2 3}", "{1 2 99}");
    evals("set-nth -1 99 {1 2 3}", "{1 2 99}");
    evals("set-nth -3 99 {1 2 3}", "{99 2 3}");
    evals("set-nth 0 {} {1}", "{{}}");
    fails("set-nth 0 99 {}", "set-nth", "0 99 {}");
    fails("set-nth -1 99 {}", "set-nth", "-1 99 {}");
    fails("set-nth 1 99 {1}", "set-nth", "1 99 {1}");
    fails("set-nth 0 99 0", "set-nth", "0 99 0");
    fails("set-nth {1 2 3} 99 0", "set-nth", "{1 2 3} 99 0");

    evals("insert-before 0 99 {1 2 3}", "{99 1 2 3}");
    evals("insert-before 2 99 {1 2 3}", "{1 2 99 3}");
    evals("insert-before 3 99 {1 2 3}", "{1 2 3 99}");
    evals("insert-before -1 99 {1 2 3}", "{1 2 99 3}");
    evals("insert-before -3 99 {1 2 3}", "{99 1 2 3}");
    evals("insert-before 0 {} {1}", "{{} 1}");
    evals("insert-before 0 99 {}", "{99}");
    evals("insert-before -1 99 {}", "{99}");
    fails("insert-before -2 99 {}", "insert-before", "-2 99 {}");
    fails("insert-before 2 99 {1}", "insert-before", "2 99 {1}");
    fails("insert-before 0 99 0", "insert-before", "0 99 0");
    fails(
        "insert-before {1 2 3} 99 {}",
        "insert-before",
        "{1 2 3} 99 {}",
    );

    evals("remove-nth 0 {1 2 3}", "{2 3}");
    evals("remove-nth 1 {1 2 3}", "{1 3}");
    evals("remove-nth 2 {1 2 3}", "{1 2}");
    evals("remove-nth -1 {1 2 3}", "{1 2}");
    evals("remove-nth -2 {1 2 3}", "{1 3}");
    evals("remove-nth 0 {1}", "{}");
    fails("remove-nth 0 {}", "remove-nth", "0 {}");
    fails("remove-nth 3 {1 2 3}", "remove-nth", "3 {1 2 3}");
    fails("remove-nth 3 0", "remove-nth", "3 0");
    fails("remove-nth {1 2 3} {}", "remove-nth", "{1 2 3} {}");

    evals("swap-remove-nth 0 {1 2 3}", "{3 2}");
    evals("swap-remove-nth 1 {1 2 3}", "{1 3}");
    evals("swap-remove-nth 2 {1 2 3}", "{1 2}");
    evals("swap-remove-nth -1 {1 2 3}", "{1 2}");
    evals("swap-remove-nth -2 {1 2 3}", "{1 3}");
    evals("swap-remove-nth 0 {1}", "{}");
    fails("swap-remove-nth 0 {}", "swap-remove-nth", "0 {}");
    fails("swap-remove-nth 3 {1 2 3}", "swap-remove-nth", "3 {1 2 3}");
    fails("swap-remove-nth 3 0", "swap-remove-nth", "3 0");
    fails(
        "swap-remove-nth {1 2 3} {}",
        "swap-remove-nth",
        "{1 2 3} {}",
    );

    evals("concat {{1 2} {3 4}}", "{1 2 3 4}");
    evals(
        "concat {{1 2} {3 4} {5} {{6} {7}} {} {} {8 {9}}}",
        "{1 2 3 4 5 {6} {7} 8 {9}}",
    );
    evals("concat {}", "{}");
    fails("concat 0", "concat", "0");
    fails("concat {0 1 2 3}", "concat", "{0 1 2 3}");

    evals("slice 0 3 {1 2 3}", "{1 2 3}");
    evals("slice 0 2 {1 2 3}", "{1 2}");
    evals("slice 1 3 {1 2 3}", "{2 3}");
    evals("slice 2 3 {1 2 3}", "{3}");
    evals("slice 2 2 {1 2 3}", "{}");
    evals("slice -2 -1 {1 2 3}", "{2}");
    evals("slice 0 0 {}", "{}");
    evals("slice -1 -1 {}", "{}");
    fails("slice 0 3 {1 2}", "slice", "0 3 {1 2}");
    fails("slice 4 3 {1 2}", "slice", "4 3 {1 2}");
    fails("slice 1 1 {}", "slice", "1 1 {}");
    fails("slice 2 0 {1 2 3}", "slice", "2 0 {1 2 3}");
    fails("slice {} 0 {1 2 3}", "slice", "{} 0 {1 2 3}");
    fails("slice 0 {} {1 2 3}", "slice", "0 {} {1 2 3}");
    fails("slice 0 0 0", "slice", "0 0 0");

    // evals("build-list {1 2 3 4}", "{1 2 3 4}");
    // evals("build-list {}", "{}");
    // evals("build-list {+ 1 2  + 2 2  + 3 3}", "{3 4 6}");
    // evals("define { iota { choose {} {iota} > 0 dup - 1 } } { iota 3 }", "{0 1 2}");
}
