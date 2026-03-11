use crate::eval::{Eval, eval, Program, ContView};
use crate::parser::parse;
use crate::val::{SymbolTable, Vals, Values};

mod builtins;
mod eval;
mod parser;
mod tests;
mod val;

fn main() {
    let mut t = SymbolTable::new();
    let mut program = Vals::new();

    for arg in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&arg).unwrap();
        let ast = parse(&src, &mut t).unwrap();
        program.extend(ast);
    }

    let mut e = Eval::new(program, t);
    while e.step() {
        // println!("{:?} | {:?}", ContView(&e.sym_table, &e.program), Values(&e.sym_table, &e.stack[..]));
        // println!("## {:?}", e.program);
    }
    println!("{:?}", Values(&e.sym_table, &e.stack[..]));
}
