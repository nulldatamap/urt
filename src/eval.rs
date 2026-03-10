use crate::builtins;
use crate::val::{Program, Sym, SymbolTable, Val, Vals, Values};
use std::collections::HashMap;
use std::fmt::Write;

pub type Builtin = fn(&mut Eval) -> bool;

pub struct Eval {
    builtins: HashMap<Sym, Builtin>,
    sym_table: SymbolTable,
    pub(crate) lexicon: Vec<HashMap<Sym, Vals>>,
    pub(crate) program: Vals,
    pub(crate) stack: Vec<Val>,
}

pub fn eval(program: Vals, t: SymbolTable) -> Result<Vals, Eval> {
    let mut e = Eval::new(program, t);
    while e.step() {}
    if e.program.is_empty() {
        e.stack.reverse();
        Ok(e.stack.into())
    } else {
        Err(e)
    }
}

pub fn trace(program: Vals, t: SymbolTable) -> Result<Vals, Eval> {
    let mut e = Eval::new(program, t);
    while e.step() {
        let mut scopes_s = String::new();
        let mut scope_s = String::new();
        for scope in &e.lexicon {
            scope_s.push_str("{ ");
            for kv in scope.iter() {
                write!(scope_s, "{} {{{:?}}} ", e.sym_table.str(*kv.0), Program(&kv.1)).unwrap();
            }
            scope_s.push_str("} ");
            scopes_s.extend(scope_s.drain(..));
        }
        println!(
            "{:?} | {:?}\t\t{}",
            Program(&e.program),
            Values(&e.stack),
            scopes_s
        )
    }
    if e.program.is_empty() {
        e.stack.reverse();
        Ok(e.stack.into())
    } else {
        Err(e)
    }
}

impl Eval {
    pub(crate) fn new(program: Vals, mut t : SymbolTable) -> Eval {
        let builtins = builtins::builtins(&mut t);
        Eval {
            sym_table: t,
            builtins,
            lexicon: vec![],
            program,
            stack: vec![],
        }
    }

    #[inline(always)]
    pub(crate) fn arity<const N: usize, F>(&mut self, f: F) -> bool
    where
        F: FnOnce(&mut Eval, [Val; N]) -> bool,
    {
        if self.stack.len() < N {
            eprintln!("Arity error, expected {} got {}", N, self.stack.len());
            false
        } else {
            let mut xs = std::array::repeat(Val::Int(0));
            for i in 0..N {
                xs[i] = self.stack.pop().unwrap();
            }
            f(self, xs)
        }
    }

    fn eval_sym(&mut self, x: Sym) -> bool {
        for m in self.lexicon.iter().rev() {
            if let Some(v) = m.get(&x) {
                self.program.extend(v.iter().cloned());
                return true;
            }
        }
        if let Some(f) = self.builtins.get(&x) {
            f(self)
        } else {
            eprintln!("Unknown symbol: `{}`", self.sym_table.str(x));
            false
        }
    }

    pub(crate) fn step(&mut self) -> bool {
        let Some(head) = self.program.pop_back() else {
            return false;
        };

        match head {
            Val::Sym(x) => {
                let ok = self.eval_sym(x);
                if !ok {
                    self.program.push_back(Val::Sym(x));
                }
                ok
            }
            other => {
                self.stack.push(other);
                true
            }
        }
    }
}
