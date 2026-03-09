use std::collections::VecDeque;
use std::fs::File;
use crate::eval::Eval;
use crate::parser::parse;
use crate::val::{Program, Val, Vals, Values};

mod builtins;
mod eval;
mod parser;
mod tests;
mod val;

fn main() {
    let mut program = Vals::new();

    for arg in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&arg).unwrap();
        let ast = parse(&src).unwrap();
        program.extend(ast);
    }

    let mut e = Eval::new(program);
    while e.step() {
        println!("{:?} | {:?}", Program(&e.program), Values(&e.stack[..]));
    }
}
