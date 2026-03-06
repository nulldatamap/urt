use bitflag::{Flags, bitflag};
use std::collections::VecDeque;
use std::fmt;
use std::iter::Rev;

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
    pub(crate) fn is_truthy(&self) -> bool {
        match self {
            Val::Int(x) => *x != 0,
            Val::Quote(x) => x.len() != 0,
            _ => true,
        }
    }
}

#[bitflag(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ValsFlags {
    None = 0,
    Empty = 1 << 0,
    // Sorted = 1 << 1,
    // OnlyBits = 1 << 2,
    // OnlyBytes = 1 << 3,
    // OnlyInts = 1 << 4,
    // SetLike = 1 << 5,
    // MapLike = 1 << 6,
}

impl ValsFlags {
    fn update_under_add_element(self, v : &Val) -> ValsFlags {
        let mut flags = self;

        let mut fresh = self.contains(Self::Empty);

        // Can't be empty if we're adding an element
        flags.unset(ValsFlags::Empty);

        /*
        // Look for possible element invariants:
        if fresh || base.contains(Self::OnlyBits) {
            // TODO:
        }
        if fresh || base.contains(Self::OnlyBytes) {
            // TODO:
        }
        if fresh || base.contains(Self::OnlyInts) {
            // TODO:
        }
        */

        flags
    }
}

impl Default for ValsFlags {
    fn default() -> Self {
        ValsFlags::None
    }
}

macro_rules! mk_iter {
    ($($name:ident $(<$lt:lifetime>)? : ($deque_ty:ty, $elm_ty:ty)),+) => (
        $(
            enum $name $(<$lt>)? {
                Nil,
                Deque($deque_ty),
            }

            impl $(<$lt>)? Iterator for $name<$($lt)?> {
                type Item = $elm_ty;

                fn next(&mut self) -> Option<Self::Item> {
                    match self {
                        $name::Nil => None,
                        $name::Deque(iter) => iter.next(),
                    }
                }
            }

            impl$(<$lt>)? DoubleEndedIterator for $name$(<$lt>)? {
                fn next_back(&mut self) -> Option<Self::Item> {
                    match self {
                        $name::Nil => None,
                        $name::Deque(iter) => iter.next_back(),
                    }
                }
            }

        )+
    );
}

mk_iter!(
    Iter<'a> : (std::collections::vec_deque::Iter<'a, Val>, &'a Val),
    IterMut<'a> : (std::collections::vec_deque::IterMut<'a, Val>, &'a mut Val),
    IntoIter : (std::collections::vec_deque::IntoIter<Val>, Val)
);

#[derive(Clone, PartialEq)]
enum ValsRepr {
    Nil,
    Deque(VecDeque<Val>),
}

impl ValsRepr {
    fn len(&self) -> usize {
        match &self {
            ValsRepr::Nil => 0,
            ValsRepr::Deque(deque) => deque.len(),
        }
    }

    fn iter(&self) -> Iter {
        match self {
            ValsRepr::Nil => Iter::Nil,
            ValsRepr::Deque(vs) => Iter::Deque(vs.iter()),
        }
    }

    fn iter_mut(&mut self) -> IterMut {
        match self {
            ValsRepr::Nil => IterMut::Nil,
            ValsRepr::Deque(vs) => IterMut::Deque(vs.iter_mut()),
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
            },
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
            },
            ValsRepr::Deque(vs) => {
                vs.push_front(v);
            }
        }
    }
}

impl IntoIterator for ValsRepr {
    type Item = Val;
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            ValsRepr::Nil => IntoIter::Nil,
            ValsRepr::Deque(deque) => IntoIter::Deque(deque.into_iter()),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Vals {
    flags: ValsFlags,
    repr: ValsRepr,
}

impl Vals {
    pub const fn empty() -> Self {
        Vals { flags: ValsFlags::None, repr: ValsRepr::Nil }
    }

    pub fn iter(&self) -> Iter {
        self.repr.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut {
        self.repr.iter_mut()
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
    }

    fn update_after_shrinking(&mut self) {
        if self.repr.len() == 0 {
            // Maybe don't throw away the memory just yet?
            // *self = Vals::empty();
            self.flags.set(ValsFlags::Empty);
        }
        self.check_invariants();
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

    pub fn push_back(&mut self, v: Val) {
        self.check_invariants();
        self.flags = self.flags.update_under_add_element(&v);
        self.repr.push_back(v);
    }

    pub fn push_front(&mut self, v: Val) {
        self.check_invariants();
        self.flags = self.flags.update_under_add_element(&v);
        self.repr.push_front(v);
    }

    pub fn pop_back(&mut self) -> Option<Val> {
        self.check_invariants();
        let r = self.repr.pop_back();
        self.update_after_shrinking();
        r
    }

    pub fn pop_front(&mut self) -> Option<Val> {
        self.check_invariants();
        let r = self.repr.pop_front();
        self.update_after_shrinking();
        r
    }
}

impl IntoIterator for Vals {
    type Item = Val;
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.repr.into_iter()
    }
}

impl Extend<Val> for Vals {
    fn extend<T: IntoIterator<Item=Val>>(&mut self, iter: T) {
        // TODO: Could be specialized
        for v in iter {
            self.push_back(v);
        }
    }
}

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
