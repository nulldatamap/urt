mod analysis;
mod rt;

use rt::eval::Eval;
use rt::parser::parse;
use rt::val::{SymbolTable, Vals};

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
        // println!(
        //     "{:?} | {:?}",
        //     e.sym_table.show(&e.program),
        //     e.sym_table.show(&e.stack)
        // );
    }
    println!("{:?}", e.sym_table.show(&e.stack));
}
