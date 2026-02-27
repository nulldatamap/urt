use crate::eval::Eval;
use crate::parser::parse;
use crate::val::{Program, Values};

mod builtins;
mod eval;
mod parser;
mod val;

fn main() {
    let input = "locals {name age x y z} {name} {John} 45 00421";
    let ast = parse(input).unwrap();
    println!("{:?} | ", Program(&ast));

    let mut e = Eval::new(ast);
    while e.step() {
        println!("{:?} | {:?}", Program(&e.program[..]), Values(&e.stack[..]));
    }
}
