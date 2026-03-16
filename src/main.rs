mod rt;

use rt::eval::Eval;
use rt::parser::parse;
use rt::val::{SymbolTable, Vals, Values};


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
