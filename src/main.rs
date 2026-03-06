use crate::eval::Eval;
use crate::parser::parse;
use crate::val::Stack;

mod builtins;
mod eval;
mod parser;
mod tests;
mod val;

fn main() {
    let input = "define { two {2} x {3} } { + two 2 }";
    let ast = parse(input).unwrap();
    println!("{:?} | ", &ast);

    let mut e = Eval::new(ast);
    while e.step() {
        println!("{:?} | {:?}", &e.program, Stack(&e.stack[..]));
    }
}
