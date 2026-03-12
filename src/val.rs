use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::fmt::Formatter;
use std::ops::Range;
use std::rc::Rc;

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
        self.symbols
            .iter()
            .find(|kv| *kv.1 == x)
            .expect("Invalid intern string")
            .0
    }
}

#[derive(Clone, Debug)]
pub struct Ref {
    vals: Rc<Vals>,
    range: Range<usize>,
}

impl Ref {
    pub fn new(vals: Vals) -> Ref {
        let n = vals.len();
        Ref {
            vals: Rc::new(vals),
            range: 0..n,
        }
    }

    pub fn len(&self) -> usize {
        self.range.len()
    }

    pub fn iter(&self) -> std::collections::vec_deque::Iter<Val> {
        self.vals.range(self.range.clone())
    }

    pub fn into_vals(self) -> Vals {
        match Rc::try_unwrap(self.vals) {
            Ok(vs) => vs,
            Err(r) => r.as_ref().range(self.range).cloned().collect::<Vals>(),
        }
    }

    pub fn pop_front(&mut self) -> Option<&Val> {
        if self.range.is_empty() {
            None
        } else {
            let v = &self.vals[self.range.start];
            self.range.start += 1;
            Some(v)
        }
    }

    pub fn pop_back(&mut self) -> Option<&Val> {
        if self.range.is_empty() {
            None
        } else {
            let v = &self.vals[self.range.end - 1];
            self.range.end -= 1;
            Some(v)
        }
    }

    pub fn slice(&mut self, range: Range<usize>) {
        self.range.start += range.start;
        self.range.end = self.range.start + range.len();
    }
}

#[derive(Clone, Debug)]
pub enum Val {
    Int(i64),
    Sym(Sym),
    Kw(Sym),
    List(Vals),
    Ref(Ref),
}

impl PartialEq for Val {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) => *i == *j,
            (Self::Sym(x), Self::Sym(y)) => *x == *y,
            (Self::Kw(x), Self::Kw(y)) => *x == *y,
            _ => {
                if self.is_list() && other.is_list() {
                    if self.len() != other.len() {
                        return false;
                    }
                    match (self, other) {
                        (Self::List(l0), Self::List(l1)) => *l0 == *l1,
                        (Self::Ref(r), Self::List(l)) | (Self::List(l), Self::Ref(r)) => {
                            r.iter().eq(l.iter())
                        }
                        (Self::Ref(r0), Self::Ref(r1)) => r0.iter().eq(r1.iter()),
                        _ => unreachable!(),
                    }
                } else {
                    false
                }
            }
        }
    }
}

pub const VAL_TRUE: Val = Val::Int(1);
pub const VAL_FALSE: Val = Val::Int(0);
pub const VAL_EMPTY: Val = Val::List(VecDeque::new());
pub const VAL_LEAVE_SCOPE: Val = Val::Sym(LEAVE_SCOPE_SYM);

impl Val {
    pub fn is_int(&self) -> bool {
        matches!(self, Val::Int(_))
    }

    pub fn is_list(&self) -> bool {
        matches!(self, Val::List(_) | Val::Ref(_))
    }

    pub fn is_sym(&self) -> bool {
        matches!(self, Val::Sym(_))
    }

    pub fn is_kw(&self) -> bool {
        matches!(self, Val::Kw(_))
    }

    pub fn len(&self) -> usize {
        match self {
            Val::List(l) => l.len(),
            Val::Ref(r) => r.len(),
            _ => panic!("Can't get the length of a non-list: {:?}", self),
        }
    }

    pub fn pop_front(&mut self) -> Option<Result<Val, &Val>> {
        match self {
            Val::List(vs) => vs.pop_front().map(Ok),
            Val::Ref(r) => r.pop_front().map(Err),
            _ => panic!("Can't `pop_front` on a non-list: {:?}", self),
        }
    }

    pub fn pop_back(&mut self) -> Option<Result<Val, &Val>> {
        match self {
            Val::List(vs) => vs.pop_back().map(Ok),
            Val::Ref(r) => r.pop_back().map(Err),
            _ => panic!("Can't `pop_back` on a non-list: {:?}", self),
        }
    }

    pub fn slice(&mut self, from: usize, to: usize) {
        match self {
            Val::List(vs) => {
                vs.drain(to..);
                vs.drain(..from);
            },
            Val::Ref(r) => {
                r.slice(from..to);
            },
            _ => panic!("Can't `slice` on a non-list: {:?}", self),
        }
    }

    pub fn nth(&self, i : usize) -> &Val {
        match self {
            Val::List(vs) => &vs[i],
            Val::Ref(r) => &r.vals[r.range.start + i],
            _ => panic!("Can't index into a non-list: {:?}", self)
        }
    }

    pub fn iter(&self) -> std::collections::vec_deque::Iter<Val> {
        match self {
            Val::List(l) => l.iter(),
            Val::Ref(r) => r.iter(),
            _ => panic!("Can't iterate over a non-list: {:?}", self),
        }
    }

    pub fn into_list(self) -> Vals {
        match self {
            Val::List(vs) => vs,
            Val::Ref(r) => r.into_vals(),
            _ => panic!("Tried to get list from {:?}", self),
        }
    }

    pub fn into_list_ref(self) -> Ref {
        match self {
            Val::List(vs) => Ref::new(vs),
            Val::Ref(r) => r,
            _ => panic!("Tried to get list from {:?}", self),
        }
    }

    pub fn into_sharable(self) -> Self {
        match self {
            Val::List(vs) => Val::Ref(Ref::new(vs)),
            v => v,
        }
    }

    pub fn list_mut(&mut self) -> &mut Vals {
        match self {
            Val::List(vs) => vs,
            Val::Ref(_) => {
                let Val::Ref(r) = std::mem::replace(self, Val::Int(0)) else {
                    unreachable!()
                };
                let vs = r.into_vals();
                *self = Val::List(vs);
                let Val::List(vs) = self else { unreachable!() };
                vs
            }
            _ => panic!("Tried to get list from {:?}", self),
        }
    }

    pub fn list_or_ref_mut(&mut self) -> Result<&mut Vals, &mut Ref> {
        match self {
            Val::List(vs) => Ok(vs),
            Val::Ref(r) => Err(r),
            _ => panic!("Tried to get list/ref from {:?}", self),
        }
    }

    pub fn sym(&self) -> Sym {
        if let Val::Sym(s) = self {
            *s
        } else {
            panic!("{:?} is not a symbol", self);
        }
    }

    pub fn kw(&self) -> Sym {
        if let Val::Kw(s) = self {
            *s
        } else {
            panic!("{:?} is not a keyword", self);
        }
    }

    pub fn int(&self) -> i64 {
        if let Val::Int(i) = self {
            *i
        } else {
            panic!("{:?} is not an int", self);
        }
    }

    pub(crate) fn is_truthy(&self) -> bool {
        match self {
            Val::Int(x) => *x != 0,
            Val::List(x) => x.len() != 0,
            Val::Ref(x) => x.len() != 0,
            _ => true,
        }
    }
}

pub type Vals = VecDeque<Val>;

// impl fmt::Debug for Val {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             Val::Int(i) => write!(f, "{i}"),
//             Val::Sym(Sym(s)) => write!(f, "##{s:X}"),
//             Val::Kw(Sym(s)) => write!(f, ":##{s:X}"),
//             _ => {
//                 let vals = match self {
//                     Val::List(vs) => vs,
//                     Val::Ref(r) => r.as_ref(),
//                     _ => unreachable!(),
//                 };
//                 write!(f, "{{")?;
//                 let mut first = true;
//                 for v in vals {
//                     if !first {
//                         write!(f, " ")?;
//                     }
//                     first = false;
//                     write!(f, "{:?}", v)?;
//                 }
//                 write!(f, "}}")
//             }
//         }
//     }
// }
//

pub struct RefProgram<'a>(pub &'a SymbolTable, pub &'a Ref);
pub struct Program<'a>(pub &'a SymbolTable, pub &'a Vals);
pub struct Values<'a>(pub &'a SymbolTable, pub &'a [Val]);
pub struct Value<'a>(pub &'a SymbolTable, pub &'a Val);

impl<'a> fmt::Debug for RefProgram<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let start = self
            .1
            .iter()
            .rposition(|x| matches!(x, Val::Sym(k) if *k == LEAVE_SCOPE_SYM))
            .map(|x| x + 1)
            .unwrap_or(0);
        let mut first = true;
        if start > 0 {
            write!(f, "... ")?;
        }
        for v in self.1.iter().skip(start) {
            if !first {
                write!(f, " ")?;
            }
            first = false;
            write!(f, "{:?} ", Value(self.0, v))?;
        }
        Ok(())
    }
}

impl<'a> fmt::Debug for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.1 {
            Val::Int(i) => write!(f, "{i}"),
            Val::Sym(s) => write!(f, "{}", self.0.str(*s)),
            Val::Kw(s) => write!(f, ":{}", self.0.str(*s)),
            Val::List(vals) => Program(self.0, vals).fmt(f),
            Val::Ref(vals) => RefProgram(self.0, vals).fmt(f),
        }
    }
}

impl<'a> fmt::Debug for Program<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let start = self
            .1
            .iter()
            .rposition(|x| matches!(x, Val::Sym(k) if *k == LEAVE_SCOPE_SYM))
            .map(|x| x + 1)
            .unwrap_or(0);
        let mut first = true;
        if start > 0 {
            write!(f, "... ")?;
        }
        for v in self.1.iter().skip(start) {
            if !first {
                write!(f, " ")?;
            }
            first = false;
            write!(f, "{:?} ", Value(self.0, v))?;
        }
        Ok(())
    }
}

impl<'a> fmt::Debug for Values<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for v in self.1.iter().rev() {
            if !first {
                write!(f, " ")?;
            }
            first = false;
            write!(f, "{:?} ", Value(self.0, v))?
        }
        Ok(())
    }
}
