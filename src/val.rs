use std::collections::VecDeque;
use std::fmt;

#[derive(PartialEq, Clone)]
pub enum Val {
    Int(i64),
    Sym(String),
    Kw(String),
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
            Val::Sym(s) => write!(f, "{s}"),
            Val::Kw(s) => write!(f, ":{s}"),
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
pub struct Values<'a>(pub &'a [Val]);

impl<'a> fmt::Debug for Program<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for v in self.0 {
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
