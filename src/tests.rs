#![cfg(test)]

use crate::eval::{eval, trace};
use crate::parser::parse;
use crate::val::{Program, Val, Vals, Values};

fn evals(program: &'static str, stack: &'static str) {
    let p = parse(program).unwrap();
    let exp = parse(stack).unwrap();
    match eval(p) {
        Ok(got) => assert_eq!(got, exp),
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

    evals("define {} {1}", "1");
    evals("define {x {1} y {2}} {+ x y}", "3");
    evals("define {x {1}} { + x define {x {2}} { x } }", "3");
    evals("define {x {1}} {x} define {x {2}} {x}", "1 2");

    fails("locals {x y} {x} 1", "locals", "{x y} {x} 1");
    fails("locals {1} {x} 1", "locals", " {1} {x} 1");
    fails("x locals {x} {x} 1", "x", "1");

    fails("define {x 1} {}", "define", " {x 1} {}");
    fails("define {x} {}", "define", " {x} {}");
    fails("x define {x {1}} {}", "x", "");
}

#[test]
fn recursion() {
    // Naive:
    // evals("define { fib { locals { x } { choose { + fib - x 1 fib - x 2 } {1} > x 2 } } }\
    // { fib 6 }\
    // ", "8");
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
