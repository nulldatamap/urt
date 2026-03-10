use crate::builtins;
use crate::val::{Program, Sym, SymbolTable, Val, Vals, Values};
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Formatter, Write};
use std::ops::Deref;
use std::rc::Rc;

pub type Builtin = fn(&mut Eval) -> bool;

pub type Ref = Rc<Vals>;

#[derive(Clone, PartialEq)]
pub enum ValOrRef {
    Val(Val),
    Ref(Ref)
}

impl ValOrRef {
    pub fn is_int(&self) -> bool {
        match self {
            ValOrRef::Val(Val::Int(_)) => true,
            _ => false,
        }
    }

    pub fn is_list(&self) -> bool {
        match self {
            ValOrRef::Val(Val::Quote(_)) | ValOrRef::Ref(_) => true,
            _ => false,
        }
    }

    pub fn is_sym(&self) -> bool {
        match self {
            ValOrRef::Val(Val::Sym(_)) => true,
            _ => false,
        }
    }

    pub fn is_kw(&self) -> bool {
        match self {
            ValOrRef::Val(Val::Kw(_)) => true,
            _ => false,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            ValOrRef::Val(x) => x.is_truthy(),
            ValOrRef::Ref(xs) => xs.len() > 0,
        }
    }
}

impl fmt::Debug for ValOrRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ValOrRef::Val(x) => x.fmt(f),
            ValOrRef::Ref(vals) => {
                write!(f, "{{")?;
                let mut first = true;
                for v in vals.iter() {
                    if !first {
                        write!(f, " ")?;
                    }
                    first = false;
                    write!(f, "{:?}", v)?;
                }
                write!(f, "}}")
            }
        }
    }
}

impl From<Val> for ValOrRef {
    fn from(value: Val) -> Self {
        ValOrRef::Val(value)
    }
}

impl From<Ref> for ValOrRef {
    fn from(value: Ref) -> Self {
        ValOrRef::Ref(value)
    }
}
impl Into<Val> for ValOrRef {
    fn into(self) -> Val {
        match self {
            ValOrRef::Val(v) => v,
            ValOrRef::Ref(v) => Val::Quote(v.deref().clone()),
        }
    }
}

pub struct Eval {
    builtins: HashMap<Sym, Builtin>,
    sym_table: SymbolTable,
    pub(crate) lexicon: Vec<HashMap<Sym, Ref>>,
    pub(crate) program: Vals,
    pub(crate) stack: Vec<ValOrRef>,
}

pub fn eval(program: Vals, t: SymbolTable) -> Result<Vals, Eval> {
    let mut e = Eval::new(program, t);
    while e.step() {}
    if e.program.is_empty() {
        Ok(e.get_stack())
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
        Ok(e.stack.into_iter().rev().map(|x| x.into()).collect())
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

    pub fn get_stack(&self) -> Vals {
        self.stack.iter().rev().map(|x| x.clone().into()).collect()
    }

    pub fn push(&mut self, v: impl Into<ValOrRef>) {
        self.stack.push(v.into());
    }

    #[inline(always)]
    pub(crate) fn arity<const N: usize, F>(&mut self, f: F) -> bool
    where
        F: FnOnce(&mut Eval, [ValOrRef; N]) -> bool,
    {
        if self.stack.len() < N {
            eprintln!("Arity error, expected {} got {}", N, self.stack.len());
            false
        } else {
            let mut xs = std::array::repeat(ValOrRef::Val(Val::Int(0)));
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
                self.stack.push(ValOrRef::Val(other));
                true
            }
        }
    }
}
