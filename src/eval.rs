use crate::builtins;
use crate::val::{Program, Val, Vals, Values};
use std::collections::HashMap;
use std::fmt::Write;

pub type Builtin = fn(&mut Eval) -> bool;

pub struct Eval {
    builtins: HashMap<&'static str, Builtin>,
    pub(crate) lexicon: Vec<HashMap<String, Vals>>,
    pub(crate) program: Vals,
    pub(crate) stack: Vec<Val>,
}

pub fn eval(program: Vals) -> Result<Vals, Eval> {
    let mut e = Eval::new(program);
    while e.step() {}
    if e.program.is_empty() {
        e.stack.reverse();
        Ok(e.stack.into())
    } else {
        Err(e)
    }
}

pub fn trace(program: Vals) -> Result<Vals, Eval> {
    let mut e = Eval::new(program);
    while e.step() {
        let mut scopes_s = String::new();
        let mut scope_s = String::new();
        for scope in &e.lexicon {
            scope_s.push_str("{ ");
            for kv in scope.iter() {
                write!(scope_s, "{} {{{:?}}} ", kv.0, Program(&kv.1)).unwrap();
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
    pub(crate) fn new(program: Vals) -> Eval {
        Eval {
            builtins: builtins::builtins(),
            lexicon: vec![],
            program,
            stack: vec![],
        }
    }

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

    pub(crate) fn lookup(&self, x: &str) -> Option<&Vals> {
        for m in self.lexicon.iter().rev() {
            if let Some(v) = m.get(x) {
                return Some(&v);
            }
        }
        None
    }

    fn eval_sym(&mut self, x: &str) -> bool {
        if let Some(v) = self.lookup(x).cloned() {
            self.program.extend(v.into_iter());
            true
        } else {
            if let Some(f) = self.builtins.get(x) {
                f(self)
            } else {
                eprintln!("Unknown symbol: `{}`", x);
                false
            }
        }
    }

    pub(crate) fn step(&mut self) -> bool {
        let Some(head) = self.program.pop_back() else {
            return false;
        };

        match head {
            Val::Sym(x) => {
                let ok = self.eval_sym(&x);
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
