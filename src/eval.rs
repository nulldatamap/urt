use crate::builtins;
use crate::val::{Program, Sym, SymbolTable, Val, Vals, Values};
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Formatter, Write};
use std::ops::Deref;
use std::rc::Rc;

pub type Builtin = fn(&mut Eval) -> bool;

pub type Ref = Rc<Vals>;

#[derive(Clone, PartialEq, Debug)]
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

    pub fn int(&self) -> i64 {
        let ValOrRef::Val(Val::Int(i)) = self else {
            panic!("Tried to get int from {:?}", self);
        };
        *i
    }

    pub fn sym(&self) -> Sym {
        let ValOrRef::Val(Val::Sym(s)) = self else {
            panic!("Tried to get int from {:?}", self);
        };
        *s
    }

    pub fn kw(&self) -> Sym {
        let ValOrRef::Val(Val::Kw(kw)) = self else {
            panic!("Tried to get int from {:?}", self);
        };
        *kw
    }

    pub fn list(&self) -> &Vals {
        match self {
            ValOrRef::Val(Val::Quote(vs)) => vs,
            ValOrRef::Ref(r) => r.as_ref(),
            _ => panic!("Tried to get list from {:?}", self)
        }
    }

    pub fn into_list(self) -> Vals {
        match self {
            ValOrRef::Val(Val::Quote(vs)) => vs,
            ValOrRef::Ref(r) => r.as_ref().clone(),
            _ => panic!("Tried to get list from {:?}", self)
        }
    }

    pub fn list_mut(&mut self) -> &mut Vals {
        match self {
            ValOrRef::Val(Val::Quote(vs)) => vs,
            ValOrRef::Ref(r) => {
                *self = ValOrRef::Val(Val::Quote(r.as_ref().clone()));
                let ValOrRef::Val(Val::Quote(r)) = self else { unreachable!() };
                r
            },
            _ => panic!("Tried to get list from {:?}", self)
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            ValOrRef::Val(x) => x.is_truthy(),
            ValOrRef::Ref(xs) => xs.len() > 0,
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
    pub(crate) sym_table: SymbolTable,
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
                write!(scope_s, "{} {{{:?}}} ", e.sym_table.str(*kv.0), Program(&e.sym_table, &kv.1)).unwrap();
            }
            scope_s.push_str("} ");
            scopes_s.extend(scope_s.drain(..));
        }
        println!(
            "{:?} | {:?}\t\t{}",
            Program(&e.sym_table, &e.program),
            Values(&e.sym_table, &e.stack),
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
