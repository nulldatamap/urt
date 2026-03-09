use bitflag::{Flags, bitflag};
use std::collections::VecDeque;
use std::fmt;
use std::ops::{Index, RangeBounds};

#[derive(PartialEq, Clone)]
pub enum Val {
    Int(i64),
    Sym(String),
    Kw(String),
    Quote(Vals),
}

pub const VAL_TRUE: Val = Val::Int(1);
pub const VAL_FALSE: Val = Val::Int(0);
pub const VAL_EMPTY: Val = Val::Quote(Vals::empty());

impl Val {
    pub fn is_truthy(&self) -> bool {
        match self {
            Val::Int(x) => *x != 0,
            Val::Quote(x) => x.len() != 0,
            _ => true,
        }
    }

    pub fn i64(&self) -> Option<i64> {
        if let Self::Int(x) = self {
            Some(*x)
        } else {
            None
        }
    }

    pub fn to_val_ref(&self) -> ValRef {
        match self {
            Val::Int(x) => ValRef::Int(*x),
            Val::Sym(x) => ValRef::Sym(&*x),
            Val::Kw(x) => ValRef::Kw(&*x),
            Val::Quote(x) => ValRef::Quote(x),
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum ValRef<'a> {
    Int(i64),
    Sym(&'a str),
    Kw(&'a str),
    Quote(&'a Vals),
}

#[bitflag(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ValsFlags {
    None = 0,
    Empty = 1 << 0,
    OnlyBytes = 1 << 1,
}

impl ValsFlags {
    fn update_under_add_element(self, v: &Val) -> ValsFlags {
        let mut flags = self;

        let fresh = self.contains(Self::Empty);

        // Can't be empty if we're adding an element
        flags.unset(ValsFlags::Empty);

        if fresh || flags.contains(Self::OnlyBytes) {
            if let Val::Int(x) = v
                && u8::try_from(*x).is_ok()
            {
                flags.set(Self::OnlyBytes);
            } else {
                flags.unset(Self::OnlyBytes);
            }
        }

        flags
    }
}

impl Default for ValsFlags {
    fn default() -> Self {
        ValsFlags::None
    }
}

#[derive(Clone)]
pub enum ValsIter<'a> {
    Nil,
    Deque(std::collections::vec_deque::Iter<'a, Val>),
    Bytes(std::slice::Iter<'a, u8>),
}

#[derive(Clone, PartialEq)]
enum ValsRepr {
    Nil,
    Deque(VecDeque<Val>),
    Bytes(Vec<u8>),
}

impl ValsRepr {
    fn is_all_bytes(&self) -> bool {
        match self {
            Self::Nil => true,
            Self::Deque(vs) => vs
                .iter()
                .all(|v| v.i64().map_or(false, |i| u8::try_from(i).is_ok())),
            Self::Bytes(_) => true,
        }
    }

    fn len(&self) -> usize {
        match &self {
            ValsRepr::Nil => 0,
            ValsRepr::Deque(vs) => vs.len(),
            ValsRepr::Bytes(bs) => bs.len(),
        }
    }

    fn vals(&self) -> ValsIter {
        match self {
            ValsRepr::Nil => ValsIter::Nil,
            ValsRepr::Deque(vs) => ValsIter::Deque(vs.iter()),
            ValsRepr::Bytes(bs) => ValsIter::Bytes(bs.iter()),
        }
    }

    fn insert(&mut self, i: usize, v: Val) {
        match self {
            ValsRepr::Nil => {
                assert_eq!(i, 0);
                let mut vs = VecDeque::with_capacity(1);
                vs.push_back(v);
                *self = ValsRepr::Deque(vs);
            }
            ValsRepr::Deque(vs) => vs.insert(i, v),
        }
    }

    // Promotion rules:
    // - If both were unspecialized before, keep them like that
    // - If it was decided that specialization invariants were kept and one of them are specialized then specialize
    // - Otherwise despecialize fully
    fn append(&mut self, flags: ValsFlags, mut other: ValsRepr)  {
        match (self, other) {
            (ValsRepr::Nil, ValsRepr::Nil) if flags.contains(ValsFlags::Empty) => {},
            (_, ValsRepr::Nil) => {},
            (ValsRepr::Nil, o) => *self = o,
            (ValsRepr::Deque(ls), ValsRepr::Deque(mut rs)) => {
                ls.append(&mut rs);
            },
            (ValsRepr::Bytes(ls), ValsRepr::Bytes(mut rs)) => {
                assert!(flags.contains(ValsFlags::OnlyBytes));
                ls.append(&mut rs);
            },
            (_, mut other) if flags.contains(ValsFlags::OnlyBytes) => {
                let (swap, vs, mut bs) = match (self, other) {
                    (ValsRepr::Deque(vs), ValsRepr::Bytes(bs)) => (false, vs, bs),
                    (ValsRepr::Bytes(bs), ValsRepr::Deque(vs)) => (true, vs, bs),
                    _ => unreachable!(),
                };

                for v in vs {
                    bs.push(v.i64().and_then(|x| u8::try_from(x).ok()).expect("Value was promised to be a byte"));
                }
                if swap {
                    self = ValsRepr::Bytes(bs);
                }
            },
            (_, mut other) if flags == ValsFlags::None => {

            },
            (_, other) => panic!("`append` invalid repr/flag permutation:\n\tFlags: {:?}\n\tLeft: {:?}\n\tRight: {:?}", flags, self, other),
        }
    }

    fn remove(&mut self, i: usize) -> Option<Val> {
        match self {
            ValsRepr::Nil => panic!("Tried to remove element from empty Vals!"),
            ValsRepr::Deque(vs) => vs.remove(i),
        }
    }

    fn swap_remove_back(&mut self, i: usize) -> Option<Val> {
        match self {
            ValsRepr::Nil => panic!("Tried to remove element from empty Vals!"),
            ValsRepr::Deque(vs) => vs.swap_remove_back(i),
        }
    }

    fn back(&self) -> Option<&Val> {
        match self {
            ValsRepr::Nil => None,
            ValsRepr::Deque(deque) => deque.back(),
        }
    }

    fn front(&self) -> Option<&Val> {
        match self {
            ValsRepr::Nil => None,
            ValsRepr::Deque(deque) => deque.front(),
        }
    }

    fn pop_back(&mut self) -> Option<Val> {
        match self {
            ValsRepr::Nil => None,
            ValsRepr::Deque(deque) => deque.pop_back(),
        }
    }

    fn pop_front(&mut self) -> Option<Val> {
        match self {
            ValsRepr::Nil => None,
            ValsRepr::Deque(deque) => deque.pop_front(),
        }
    }

    fn push_back(&mut self, v: Val) {
        match self {
            ValsRepr::Nil => {
                let mut elms = VecDeque::with_capacity(1);
                elms.push_back(v);
                *self = ValsRepr::Deque(elms);
            }
            ValsRepr::Deque(vs) => {
                vs.push_back(v);
            }
        }
    }

    fn push_front(&mut self, v: Val) {
        match self {
            ValsRepr::Nil => {
                let mut elms = VecDeque::with_capacity(1);
                elms.push_front(v);
                *self = ValsRepr::Deque(elms);
            }
            ValsRepr::Deque(vs) => {
                vs.push_front(v);
            }
        }
    }

    fn try_get(&self, i: usize) -> Option<ValRef> {
        match self {
            ValsRepr::Nil => None,
            ValsRepr::Deque(vs) => vs.get(i).map(|x| x.to_val_ref()),
            ValsRepr::Bytes(bs) => bs.get(i).map(|b| ValRef::Int(*b as i64)),
        }
    }

    fn get(&self, i: usize) -> ValRef {
        self.try_get(i).expect("Out of bounds access")
    }
}

#[derive(Clone)]
pub struct Vals {
    flags: ValsFlags,
    repr: ValsRepr,
}

impl Vals {
    pub const fn empty() -> Self {
        Vals {
            flags: ValsFlags::Empty,
            repr: ValsRepr::Nil,
        }
    }

    pub fn with_capacity(cap: usize) -> Vals {
        Vals {
            flags: ValsFlags::Empty,
            repr: ValsRepr::Deque(VecDeque::with_capacity(cap)),
        }
    }

    pub fn vals(&self) -> ValsIter {
        self.repr.vals()
    }

    #[cfg(debug_assertions)]
    fn check_invariants(&self) {
        let is_empty_flags = self.flags.contains(ValsFlags::Empty);
        let is_empty = match &self.repr {
            ValsRepr::Nil => true,
            ValsRepr::Deque(deque) => deque.is_empty(),
            _ => false,
        };
        assert_eq!(
            is_empty, is_empty_flags,
            "Invalid invariant for `{:?}`:\n\tempty flag: {}\n\tBut is_empty(): {}",
            self, is_empty_flags, is_empty
        );

        if self.flags.contains(ValsFlags::OnlyBytes) {
            assert!(self.repr.is_all_bytes());
        }
    }

    fn update_flags_after_shrinking(&mut self) {
        if self.repr.len() == 0 {
            // Maybe don't throw away the memory just yet?
            // *self = Vals::empty();
            self.flags.set(ValsFlags::Empty);
        }
        self.check_invariants();
    }

    fn update_flags_after_add(&mut self, v: &Val) {
        self.flags = self.flags.update_under_add_element(v);
    }

    pub fn is_empty(&self) -> bool {
        self.check_invariants();

        self.flags.contains(ValsFlags::Empty)
    }

    pub fn len(&self) -> usize {
        self.check_invariants();

        if self.flags.contains(ValsFlags::Empty) {
            0
        } else {
            self.repr.len()
        }
    }

    pub fn try_get(&self, i: usize) -> Option<ValRef> {
        self.repr.try_get(i)
    }

    pub fn get(&self, i: usize) -> ValRef {
        self.repr.try_get(i).expect("Out of bounds access")
    }

    pub fn insert(&mut self, i: usize, v: Val) {
        self.check_invariants();

        self.update_flags_after_add(&v);
        self.repr.insert(i, v);
        self.check_invariants();
    }

    pub fn remove(&mut self, i: usize) {
        self.repr.remove(i);
        self.update_flags_after_shrinking();
        self.check_invariants();
    }

    pub fn swap_remove_back(&mut self, i: usize) {
        self.repr.swap_remove_back(i);
        self.update_flags_after_shrinking();
        self.check_invariants();
    }

    pub fn back(&self) -> Option<&Val> {
        self.repr.back()
    }

    pub fn front(&self) -> Option<&Val> {
        self.repr.front()
    }

    pub fn push_back(&mut self, v: Val) {
        self.check_invariants();
        self.update_flags_after_add(&v);
        self.repr.push_back(v);
        self.check_invariants();
    }

    pub fn push_front(&mut self, v: Val) {
        self.check_invariants();
        self.update_flags_after_add(v);
        self.repr.push_front(v);
        self.check_invariants();
    }

    pub fn pop_back(&mut self) -> Option<Val> {
        self.check_invariants();
        let r = self.repr.pop_back();
        self.update_flags_after_shrinking();
        self.check_invariants();
        r
    }

    pub fn pop_front(&mut self) -> Option<Val> {
        self.check_invariants();
        let r = self.repr.pop_front();
        self.update_flags_after_shrinking();
        self.check_invariants();
        r
    }

    pub fn slice(&mut self, from: usize, to: usize) {
        self.check_invariants();
        assert!(to <= self.repr.len());
        assert!(from <= self.repr.len());
        assert!(from <= to);
        _ = self.drain(to..);
        _ = self.drain(..from);
        self.update_flags_after_shrinking();
        self.check_invariants();
    }
}

impl PartialEq<Self> for Vals {
    fn eq(&self, other: &Self) -> bool {
        if self.is_empty() && other.is_empty() {
            true
        } else {
            self.flags == other.flags && self.repr == other.repr
        }
    }
}

/*
impl Index<usize> for Vals {
    type Output = Val;

    fn index(&self, index: usize) -> &Self::Output {
        self.repr.index(index)
    }
}

impl IndexMut<usize> for Vals {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.repr.index_mut(index)
    }
}

impl Extend<Val> for Vals {
    fn extend<T: IntoIterator<Item = Val>>(&mut self, iter: T) {
        // TODO: Could be specialized
        for v in iter {
            self.push_back(v);
        }
    }
}
 */

impl From<VecDeque<Val>> for Vals {
    fn from(vals: VecDeque<Val>) -> Self {
        if vals.is_empty() {
            Vals::empty()
        } else {
            let mut flags = ValsFlags::empty();
            for v in &vals {
                flags = flags.update_under_add_element(v);
            }
            let r = Vals {
                flags,
                repr: ValsRepr::Deque(vals),
            };
            r.check_invariants();
            r
        }
    }
}

impl From<Vec<Val>> for Vals {
    fn from(vals: Vec<Val>) -> Self {
        if vals.is_empty() {
            Vals::empty()
        } else {
            let mut flags = ValsFlags::empty();
            for v in &vals {
                flags = flags.update_under_add_element(v);
            }
            let r = Vals {
                flags,
                repr: ValsRepr::Deque(VecDeque::from(vals)),
            };
            r.check_invariants();
            r
        }
    }
}

impl<const N: usize> From<[Val; N]> for Vals {
    fn from(vals: [Val; N]) -> Self {
        if vals.is_empty() {
            Vals::empty()
        } else {
            let mut flags = ValsFlags::empty();
            for v in &vals {
                flags = flags.update_under_add_element(v);
            }
            let r = Vals {
                flags,
                repr: ValsRepr::Deque(VecDeque::from(vals)),
            };
            r.check_invariants();
            r
        }
    }
}

impl Into<Vec<Val>> for Vals {
    fn into(self) -> Vec<Val> {
        self.repr.into_iter().collect()
    }
}

impl fmt::Debug for ValsRepr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValsRepr::Nil => write!(f, "{{}}"),
            ValsRepr::Deque(vs) => {
                write!(f, "{{")?;
                let mut first = true;
                for v in vs {
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

impl fmt::Debug for Vals {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.repr.fmt(f)
    }
}

impl fmt::Debug for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Val::Int(i) => write!(f, "{i}"),
            Val::Sym(s) => write!(f, "{s}"),
            Val::Kw(s) => write!(f, ":{s}"),
            Val::Quote(vals) => vals.fmt(f),
        }
    }
}

pub struct Stack<'a>(pub &'a [Val]);

impl<'a> fmt::Debug for Stack<'a> {
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
