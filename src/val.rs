use std::collections::{HashMap, VecDeque};
use std::collections::hash_map::Entry;
use std::fmt;
use crate::eval::ValOrRef;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Sym(u64);
pub const LEAVE_SCOPE_SYM: Sym = Sym(1);
pub const INT_SYM: Sym = Sym(2);
pub const LIST_SYM: Sym = Sym(3);
pub const SYMBOL_SYM: Sym = Sym(4);
pub const KEYWORD_SYM: Sym = Sym(5);

pub struct SymbolTable {
    next: u64,
    symbols: HashMap<String, Sym>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut t = SymbolTable {
            next: 1,
            symbols: HashMap::new(),
        };
        assert_eq!(t.intern("%{leave-scope}".to_string()), LEAVE_SCOPE_SYM);
        assert_eq!(t.intern("int".to_string()), INT_SYM);
        assert_eq!(t.intern("list".to_string()), LIST_SYM);
        assert_eq!(t.intern("symbol".to_string()), SYMBOL_SYM);
        assert_eq!(t.intern("keyword".to_string()), KEYWORD_SYM);
        t
    }

    pub fn intern(&mut self, x: String) -> Sym {
        match self.symbols.entry(x) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(e) => {
                let id = Sym(self.next);
                self.next += 1;
                e.insert(id);
                id
            }
        }
    }

    pub fn str(&self, x: Sym) -> &str {
        self.symbols.iter().find(|kv| *kv.1 == x).expect("Invalid intern string").0
    }
}

#[derive(PartialEq, Clone)]
pub enum Val {
    Int(i64),
    Sym(Sym),
    Kw(Sym),
    Quote(Vals),
}

pub const VAL_TRUE: Val = Val::Int(1);
pub const VAL_FALSE: Val = Val::Int(0);
pub const VAL_EMPTY: Val = Val::Quote(VecDeque::new());

impl Val {
    pub(crate) fn is_truthy(&self) -> bool {
        match self {
            Val::Int(x) => *x != 0,
            Val::Quote(x) => x.len() != 0,
            _ => true,
        }
    }
}

pub type Vals = VecDeque<Val>;

impl fmt::Debug for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Val::Int(i) => write!(f, "{i}"),
            Val::Sym(s) => write!(f, "{s:?}"),
            Val::Kw(s) => write!(f, ":{s:?}"),
            Val::Quote(vals) => {
                write!(f, "{{")?;
                let mut first = true;
                for v in vals {
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

pub struct Program<'a>(pub &'a Vals);
pub struct Values<'a>(pub &'a [ValOrRef]);

impl<'a> fmt::Debug for Program<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let start = self
            .0
            .iter()
            .rposition(|x| matches!(x, Val::Sym(k) if *k == LEAVE_SCOPE_SYM))
            .map(|x| x + 1)
            .unwrap_or(0);
        let mut first = true;
        if start > 0 {
            write!(f, "... ")?;
        }
        for v in self.0.iter().skip(start) {
            if !first {
                write!(f, " ")?;
            }
            first = false;
            write!(f, "{:?}", v)?;
        }
        Ok(())
    }
}

impl<'a> fmt::Debug for Values<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for v in self.0.iter().rev() {
            if !first {
                write!(f, " ")?;
            }
            first = false;
            write!(f, "{:?}", v)?;
        }
        Ok(())
    }
}
