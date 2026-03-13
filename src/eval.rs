use crate::builtins;
pub(crate) use crate::val::{
    Program, Ref, Sym, SymbolTable, Val, Vals, Value, LEAVE_SCOPE_SYM, VAL_LEAVE_SCOPE,
};
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Formatter, Write};
use micromap::Map;

pub type Builtin = fn(&mut Eval) -> bool;

#[derive(Debug)]
pub enum Continuation {
    Vals(Vals),
    Chunks(Vec<Cont>),
}

pub enum ContinuationIter<'a> {
    Vals(std::collections::vec_deque::Iter<'a, Val>),
    Chunks(std::slice::Iter<'a, Cont>, Option<std::collections::vec_deque::Iter<'a, Val>>),
}

impl<'a> Iterator for ContinuationIter<'a> {
    type Item = &'a Val;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Vals(it) => it.next(),
            Self::Chunks(it, sub_it) => {
                let mut new_sub_it = None;
                let r;
                if let Some(sub_it) = sub_it {
                    r = sub_it.next();

                    if r.is_none() {
                        new_sub_it = Some(None);
                    }
                } else {
                    if let Some(c) = it.next() {
                        match c {
                            Cont::LeaveScope => r = Some(&VAL_LEAVE_SCOPE),
                            Cont::Ref(vs) => {
                                let mut sub_it = vs.iter();
                                r = sub_it.next();

                                new_sub_it = Some(Some(sub_it));
                            }
                        }
                    } else {
                        r = None;
                    }
                }
                if let Some(new_sub_it) = new_sub_it {
                    *sub_it = new_sub_it;
                }
                r
            }
        }
    }
}

impl Continuation {
    fn is_empty(&self) -> bool {
        match self {
            Continuation::Vals(vs) => vs.is_empty(),
            Continuation::Chunks(cs) => cs.is_empty(),
        }
    }

    pub(crate) fn extend(&mut self, next: Ref) {
        if next.len() == 0 {
            return;
        }

        match self {
            Self::Chunks(cs) => {
                cs.push(Cont::Ref(next));
            }
            Self::Vals(vs) => vs.extend(next.iter().cloned()),
        }
    }

    pub fn leave_scope(&mut self) {
        match self {
            Self::Vals(vs) => {
                vs.push_back(VAL_LEAVE_SCOPE.clone());
            }
            Self::Chunks(cs) => {
                cs.push(Cont::LeaveScope);
            }
        }
    }

    pub fn in_tail_position(&self) -> bool {
        match self {
            Self::Vals(vs) => vs.back() == Some(&VAL_LEAVE_SCOPE),
            Self::Chunks(cs) => {
                let Some(head) = cs.last() else { return false };
                match head {
                    Cont::LeaveScope => true,
                    Cont::Ref(vs) => vs.iter().last() == Some(&VAL_LEAVE_SCOPE),
                }
            }
        }
    }

    fn undo(&mut self, v: Val) {
        match self {
            Continuation::Vals(vs) => vs.push_back(v),
            Continuation::Chunks(cs) => {
                // TODO: Could optimize the case where we're undoing a chuck
                cs.push(Cont::Ref(Ref::new([v].into())));
            }
        }
    }

    fn step(&mut self) -> Option<Val> {
        loop {
            let r = match self {
                Self::Vals(vs) => vs.pop_back(),
                Self::Chunks(cs) => {
                    let Some(head) = cs.last_mut() else {
                        return None;
                    };
                    match head {
                        Cont::LeaveScope => {
                            _ = cs.pop();
                            Some(Val::Sym(LEAVE_SCOPE_SYM))
                        }
                        Cont::Ref(vs) => {
                            let Some(r) = vs.pop_back().cloned() else {
                                _ = cs.pop();
                                continue
                            };
                            if vs.len() == 0 {
                                _ = cs.pop();
                            }
                            Some(r)
                        }
                    }
                }
            };
            return r
        }
    }

    pub fn iter(&self) -> ContinuationIter {
        match self {
            Continuation::Vals(vs) => ContinuationIter::Vals(vs.iter()),
            Continuation::Chunks(cs) => ContinuationIter::Chunks(cs.iter(), None),
        }
    }
}

#[derive(Debug)]
enum Cont {
    LeaveScope,
    Ref(Ref),
}

pub struct ContView<'a>(pub &'a SymbolTable, pub &'a Continuation);

impl<'a> fmt::Debug for ContView<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.1 {
            Continuation::Vals(vs) => Program(self.0, vs).fmt(f),
            Continuation::Chunks(cs) => {
                let start = cs
                    .iter()
                    .rposition(|x| matches!(x, Cont::LeaveScope))
                    .map(|x| x + 1)
                    .unwrap_or(0);
                let mut first = true;
                if start > 0 {
                    write!(f, "...")?;
                }
                for c in &cs[start..] {
                    if !first {
                        write!(f, " ")?;
                    }
                    match c {
                        Cont::LeaveScope => unreachable!(),
                        Cont::Ref(v) => {
                            for v in v.iter() {
                                write!(f, "{:?} ", Value(self.0, v))?;
                            }
                        }
                    }
                    first = false;
                }
                Ok(())
            }
        }
    }
}

pub(crate) enum Slot {
    Quote(Ref),
    Val(Val),
}

const SMALL_MAP_SIZE: usize = 16;

pub enum LexiconScope {
    Large(HashMap<Sym, Slot>),
    Small(Map<Sym, Slot, SMALL_MAP_SIZE>),
}

impl LexiconScope {
    pub fn new(max_size: usize) -> LexiconScope {
        if max_size <= SMALL_MAP_SIZE {
            Self::Small(Map::new())
        } else {
            Self::Large(HashMap::with_capacity(max_size))
        }
    }

    pub fn get(&self, x: Sym) -> Option<&Slot> {
        match self {
            LexiconScope::Large(m) => m.get(&x),
            LexiconScope::Small(m) => m.get(&x),
        }
    }

    pub fn insert(&mut self, x: Sym, v: Slot) -> Option<Slot> {
        match self {
            LexiconScope::Large(m) => m.insert(x, v),
            LexiconScope::Small(m) => m.insert(x, v),
        }
    }
}

pub struct Eval {
    builtins: HashMap<Sym, Builtin>,
    pub(crate) sym_table: SymbolTable,
    pub(crate) lexicon: Vec<LexiconScope>,
    pub(crate) program: Continuation,
    pub(crate) stack: Vec<Val>,
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

// pub fn trace(program: Vals, t: SymbolTable) -> Result<Vals, Eval> {
//     let mut e = Eval::new(program, t);
//     while e.step() {
//         let mut scopes_s = String::new();
//         let mut scope_s = String::new();
//         for scope in &e.lexicon {
//             scope_s.push_str("{ ");
//             for kv in scope.iter() {
//                 write!(
//                     scope_s,
//                     "{} {{{:?}}} ",
//                     e.sym_table.str(*kv.0),
//                     Program(&e.sym_table, &kv.1)
//                 )
//                 .unwrap();
//             }
//             scope_s.push_str("} ");
//             scopes_s.extend(scope_s.drain(..));
//         }
//         println!(
//             "{:?} | {:?}\t\t{}",
//             ContView(&e.sym_table, &e.program),
//             Values(&e.sym_table, &e.stack),
//             scopes_s
//         )
//     }
//     if e.program.is_empty() {
//         e.stack.reverse();
//         Ok(e.stack.into_iter().rev().map(|x| x.into()).collect())
//     } else {
//         Err(e)
//     }
// }

impl Eval {
    pub(crate) fn new(program: Vals, mut t: SymbolTable) -> Eval {
        let builtins = builtins::builtins(&mut t);
        let cont = Continuation::Chunks(vec![Cont::Ref(Ref::new(program))]);
        Eval {
            sym_table: t,
            builtins,
            lexicon: vec![],
            program: cont,
            stack: vec![],
        }
    }

    pub fn get_stack(&self) -> Vals {
        self.stack.iter().rev().map(|x| x.clone().into()).collect()
    }

    pub fn push(&mut self, v: impl Into<Val>) {
        self.stack.push(v.into());
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
            if let Some(v) = m.get(x) {
                match v {
                    Slot::Quote(v) => self.program.extend(v.clone()),
                    Slot::Val(v) => self.stack.push(v.clone()),
                }

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
        let Some(head) = self.program.step() else {
            return false;
        };

        match head {
            Val::Sym(x) => {
                let ok = self.eval_sym(x);
                if !ok {
                    self.program.undo(Val::Sym(x));
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
