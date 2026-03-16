mod tests;

use crate::analysis::AnalysisError::StackMismatch;
use crate::rt::eval::{Sym, Val, Vals};
use crate::rt::val::SymbolTable;
use std::collections::{HashMap, VecDeque};

#[derive(Debug)]
pub enum AnalysisError {
    LexicallyUndefinedSymbol(String),
    StackMismatch(String, VirtualStack, StackUsage),
    TypeMismatch(String, StackUsage, StackUsage),
    AmbiguousType(VirtualStack, StackUsage),
}

trait ArrayLikeIter<'a, T: 'a>: Iterator<Item=&'a T> + ExactSizeIterator + DoubleEndedIterator {
}

impl<'a, T> ArrayLikeIter<'a, T> for std::collections::vec_deque::Iter<'a, T> {}
impl<'a, T> ArrayLikeIter<'a, T> for std::slice::Iter<'a, T> {}

type Result<T> = std::result::Result<T, AnalysisError>;
type AnalysisResult = Result<StackUsage>;

type BuiltinInfo = HashMap<Sym, BuiltinInfoEntry>;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum BasicType {
    Int,
    Sym,
    Kw,
    List,
}

impl BasicType {
    fn matches(&self, v: &Val) -> bool {
        match self {
            BasicType::Int => v.is_int(),
            BasicType::Sym => v.is_sym(),
            BasicType::Kw => v.is_kw(),
            BasicType::List => v.is_list(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum TypeInfo {
    Basic(BasicType),
    HomList(BasicType),
    Code(Option<StackUsage>),
}

impl TypeInfo {
    const fn int() -> TypeInfo {
        TypeInfo::Basic(BasicType::Int)
    }

    const fn bool() -> TypeInfo {
        TypeInfo::Basic(BasicType::Int)
    }

    const fn anycode() -> TypeInfo {
        TypeInfo::Code(None)
    }

    const fn syms() -> TypeInfo {
        TypeInfo::HomList(BasicType::Sym)
    }

    fn matches(&self, v: &Val) -> bool {
        match (self, v) {
            (TypeInfo::Basic(bt), v) => bt.matches(v),
            (TypeInfo::HomList(bt), v) => {
                v.is_list() && v.iter().all(|x| bt.matches(x))
            },
            // TODO:
            (TypeInfo::Code(None), v) => v.is_list(),
            (TypeInfo::Code(Some(bt)), v) => unimplemented!(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum StackParam {
    Any,
    Typed(TypeInfo),
    Eval(usize),
}

impl StackParam {
    fn matches(&self, other: &Self) -> bool {
        match (self, other) {
            (StackParam::Any, _) | (_, StackParam::Any) => true,
            (x, y) => x == y,
        }
    }

    fn matches_virtual(&self, other: &VirtualVal) -> bool {
        match (other, self) {
            (VirtualVal::Virtual(p), _) => self.matches(p),
            (VirtualVal::Concrete(v), StackParam::Any) => true,
            (VirtualVal::Concrete(v), StackParam::Typed(ti)) =>
                ti.matches(v)
            ,
            _ => false,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct StackUsage {
    stack_in: Vec<StackParam>,
    stack_out: Vec<StackParam>,
}

impl StackUsage {
    fn matches(&self, other: &StackUsage) -> bool {
        self.stack_in
            .iter()
            .rev()
            .zip(other.stack_in.iter().rev())
            .all(|(x, y)| x.matches(y))
            && self
                .stack_out
                .iter()
                .rev()
                .zip(other.stack_out.iter().rev())
                .all(|(x, y)| x.matches(y))
    }

    fn matches_exact(&self, other: &StackUsage) -> bool {
        self.stack_out.len() == other.stack_out.len()
            && self.stack_in.len() == other.stack_in.len()
            && self.matches(other)
    }

    fn usage_matches_virtual<'a>(su: impl ArrayLikeIter<'a, StackParam>, other: impl ArrayLikeIter<'a, VirtualVal>) -> bool {
        let su = su.into_iter();
        let other = other.into_iter();
        su.len() <= other.len()
            && su
                .rev()
                .zip(other.rev())
                .all(|(x, y)| x.matches_virtual(y))
    }

    fn in_matches_virtual<'a>(&'a self, other: impl ArrayLikeIter<'a, VirtualVal>) -> bool {
        Self::usage_matches_virtual(self.stack_in.iter(), other)
    }

    fn out_matches_virtual<'a>(&'a self, other: impl ArrayLikeIter<'a, VirtualVal>) -> bool {
        Self::usage_matches_virtual(self.stack_out.iter(), other)
    }

    fn in_len(&self) -> usize {
        self.stack_in.len()
    }

    fn out_len(&self) -> usize {
        self.stack_out.len()
    }
}

impl StackUsage {
    pub fn new(sin: Vec<StackParam>, sout: Vec<StackParam>) -> StackUsage {
        StackUsage {
            stack_in: sin,
            stack_out: sout,
        }
    }

    pub fn from_in_out_sizes(sin: usize, sout: usize) -> StackUsage {
        StackUsage {
            stack_in: vec![StackParam::Any; sin],
            stack_out: vec![StackParam::Any; sout],
        }
    }
}

struct BuiltinInfoEntry {
    name: Sym,
    stack_usage: StackUsage,
}

impl BuiltinInfoEntry {
    fn new(s: Sym, usage: StackUsage) -> BuiltinInfoEntry {
        BuiltinInfoEntry {
            name: s,
            stack_usage: usage,
        }
    }
}

struct LocalInfo {
    stack_usage: StackUsage,
}

#[derive(Clone, Debug)]
enum VirtualVal {
    Concrete(Val),
    Virtual(StackParam),
}

type VirtualStack = VecDeque<VirtualVal>;

struct Ctx<'a> {
    t: &'a SymbolTable,
    arguments: Option<Vec<StackParam>>,
    virtual_stack: VirtualStack,
    builtin_info: &'a BuiltinInfo,
    lexical_scope: Vec<HashMap<Sym, LocalInfo>>,
}

macro_rules! stack_usage {
    // Entry
    (($($sin:tt)*) -- ($($sout:tt)*)) => (
        StackUsage::new(
            stack_usage!(stack_param [] $($sin)*)
        ,
            stack_usage!(stack_param [] $($sout)*)
        )
    );
    // Params
    (stack_param [$($res:tt)*]) => (vec![$($res)*]);
    (stack_param [$($res:tt)*] any $($rest:tt)*) => (
        stack_usage!(stack_param
          [$($res)* StackParam::Any, ]
          $($rest)*
        )
    );
    (stack_param [$($res:tt)*] eval($x:literal) $($rest:tt)*) => (
        stack_usage!(stack_param
          [$($res)* StackParam::Eval($x), ]
          $($rest)*
        )
    );
    (stack_param [$($res:tt)*] $t:ident ($($xs:expr),+) $($rest:tt)*) => (
        stack_usage!(stack_param
          [$($res)* StackParam::Typed(stack_usage!(type $t ($($xs),+))), ]
          $($rest)*
        )
    );
    (stack_param [$($res:tt)*] $t:ident $($rest:tt)*) => (
        stack_usage!(stack_param
          [$($res)* StackParam::Typed(stack_usage!(type $t)), ]
          $($rest)*
        )
    );
    // Types
    (type $t:ident) => (
        TypeInfo::$t()
    );
    (type $t:ident($($xs:expr),+)) => (
        TypeInfo::$t($($xs),+)
    );
}

pub(crate) use stack_usage;

fn builtin_info(t: &SymbolTable) -> BuiltinInfo {
    let mut builtin_info = HashMap::new();
    macro_rules! reg {
        ($($name:literal ($($sin:tt)*) -- ($($sout:tt)*));+;) => (
            $(
                if let Some(s) = t.try_get($name) {
                    builtin_info.insert(s, BuiltinInfoEntry::new(s, stack_usage!(($($sin)*) -- ($($sout)*))));
                }
            )+
        );
    }

    reg!(
        "drop" (any) -- ();
        // TODO: have a arg(X) so we can track types across these ops:
        "dup"  (any) -- (any any);
        "swap" (any any) -- (any any);

        "+" (int int) -- (int);
        "-" (int int) -- (int);
        "*" (int int) -- (int);
        "/" (int int) -- (int);
        "%" (int int) -- (int);

        "="  (any any) -- (bool);
        "<>" (any any) -- (bool);
        "<"  (int int) -- (bool);
        "<=" (int int) -- (bool);
        ">"  (int int) -- (bool);
        ">=" (int int) -- (bool);

        "true"  () -- (bool);
        "false" () -- (bool);
        "&&"    (any any) -- (any);
        "||"    (any any) -- (any);
        "not"   (any any) -- (bool);

        "unquote" (anycode) -- (eval(0));

        "%{leave-scope}" () -- ();
        "locals" (syms anycode) -- (eval(0));
    );

    builtin_info
}

impl<'a> Ctx<'a> {
    fn new(t: &'a SymbolTable, bi: &'a BuiltinInfo) -> Ctx<'a> {
        Ctx {
            t,
            arguments: None,
            virtual_stack: VecDeque::new(),
            builtin_info: bi,
            lexical_scope: vec![],
        }
    }

    fn lexical_lookup(&self, s: Sym) -> Option<&LocalInfo> {
        for scope in self.lexical_scope.iter().rev() {
            if let Some(r) = scope.get(&s) {
                return Some(r);
            }
        }

        None
    }

    fn stack_depth(&self) -> usize {
        self.virtual_stack.len()
    }

    fn uses(&mut self, s: Sym, stack_usage: &StackUsage) -> Result<()> {

        if stack_usage.in_len() > self.virtual_stack.len()
            && let Some(args) = &mut self.arguments {
            let new_args = stack_usage.stack_in[0..(stack_usage.in_len() - self.virtual_stack.len())].iter().cloned();
            args.extend(new_args.clone());
            for arg in new_args {
                self.virtual_stack.push_front(VirtualVal::Virtual(arg));
            }
        }

        if !stack_usage.in_matches_virtual(self.virtual_stack.iter()) {
            return Err(StackMismatch(
                self.t.str(s).to_string(),
                self.virtual_stack.clone(),
                stack_usage.clone(),
            ));
        }
        let mut code_vals = Vec::new();
        for sp in stack_usage.stack_in.iter() {
            if let StackParam::Typed(TypeInfo::Code(su)) = sp {
                let v =self.virtual_stack.pop_back().unwrap();
                if let VirtualVal::Concrete(v) = v {
                    let code_su = self.analyze(v.iter(), false)?;
                    if let Some(su) = su {
                        if !su.matches(&code_su) {
                            return Err(AnalysisError::TypeMismatch(format!("eval({}) of {}", code_vals.len(), self.t.str(s).to_string()), code_su.clone(), su.clone()))
                        }
                        code_vals.push(code_su);
                    }
                } else {
                    return Err(AnalysisError::AmbiguousType(self.virtual_stack.clone(), stack_usage.clone()))
                }
            } else {
                self.virtual_stack.pop_back().unwrap();
            }
        }
        for p in stack_usage.stack_out.iter() {
            self.virtual_stack.push_back(VirtualVal::Virtual(p.clone()));
        }
        Ok(())
    }

    fn analyze_sym(&mut self, s: Sym) -> Result<()> {
        if let Some(v) = self.lexical_lookup(s) {
            unimplemented!()
        } else {
            if let Some(bi) = self.builtin_info.get(&s) {
                self.uses(s, &bi.stack_usage)?;
                Ok(())
            } else {
                Err(AnalysisError::LexicallyUndefinedSymbol(
                    self.t.str(s).to_string(),
                ))
            }
        }
    }

    fn analyze<'p>(&mut self, program: impl ArrayLikeIter<'p, Val>, constrained: bool) -> AnalysisResult {
        let old_args = self.arguments.take();
        self.arguments = if constrained {
            None
        } else {
            Some(vec![])
        };
        for step in program.rev() {
            match step {
                Val::Int(_) | Val::Kw(_) | Val::List(_) | Val::Ref(_) => {
                    self.virtual_stack.push_back(VirtualVal::Concrete(step.clone()));
                }
                Val::Sym(s) => self.analyze_sym(*s)?,
            }
        }
        let mut args = self.arguments.take().unwrap_or(Vec::new());
        args.reverse();
        self.arguments = old_args;
        Ok(StackUsage {
            stack_in: args,
            stack_out: vec![StackParam::Any; self.stack_depth()],
        })
    }
}

pub fn analyze(t: &SymbolTable, program: &Vals) -> AnalysisResult {
    let bi = builtin_info(t);
    Ctx::new(t, &bi).analyze(program.iter(), true)
}
