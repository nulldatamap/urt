use crate::eval::{Builtin, Eval, LexiconScope, Slot, Value};
use crate::val::{
    Sym, SymbolTable, Val, Vals, INT_SYM, KEYWORD_SYM,
    LIST_SYM, SYMBOL_SYM, VAL_FALSE, VAL_TRUE,
};
use std::collections::HashMap;

pub fn builtins(t: &mut SymbolTable) -> HashMap<Sym, Builtin> {
    let mut b = HashMap::<Sym, Builtin>::new();
    let mut reg = |k: &'static str, f| {
        b.insert(t.intern(k.to_string()), f);
    };

    // Stack
    reg("drop", b_drop);
    reg("dup", b_dup);
    reg("swap", b_swap);
    // Arithmetic
    reg("+", b_add);
    reg("-", b_sub);
    reg("*", b_mul);
    reg("/", b_div);
    reg("%", b_mod);
    // Comparison
    reg("=", b_eq);
    reg("<>", b_neq);
    reg("<", b_le);
    reg("<=", b_leq);
    reg(">", b_ge);
    reg(">=", b_geq);
    // Logic
    reg("true", b_true);
    reg("false", b_false);
    reg("&&", b_and);
    reg("||", b_or);
    reg("not", b_not);
    // Data structure:
    reg("length", b_length);
    reg("append", b_append);
    reg("push-back", b_push_back);
    reg("push-front", b_push_front);
    reg("head", b_head);
    reg("tail", b_tail);
    reg("init", b_init);
    reg("last", b_last);
    reg("head-tail", b_head_tail);
    reg("last-init", b_last_init);
    reg("nth", b_nth);
    reg("set-nth", b_set_nth);
    reg("insert-before", b_insert_before);
    reg("remove-nth", b_remove_nth);
    reg("swap-remove-nth", b_swap_remove_nth);
    reg("concat", b_concat);
    reg("slice", b_slice);
    // reg("build-list", b_build_list);
    // Types
    reg("type-of", b_type_of);
    reg("int?", b_is_int);
    reg("symbol?", b_is_symbol);
    reg("keyword?", b_is_keyword);
    reg("list?", b_is_list);
    // Quoting
    reg("quote", b_quote);
    reg("unquote", b_unquote);
    // Control flow
    reg("choose", b_choose);
    // Scope
    reg("%{leave-scope}", b_leave_scope);
    reg("locals", b_locals);
    reg("define", b_define);

    b
}

fn b_drop(e: &mut Eval) -> bool {
    e.arity(|_e, [_]| true)
}
fn b_dup(e: &mut Eval) -> bool {
    e.arity(|e, [x]| {
        e.stack.extend([x.clone(), x]);
        true
    })
}

fn b_swap(e: &mut Eval) -> bool {
    e.arity(|e, [x, y]| {
        e.stack.extend([x, y]);
        true
    })
}

macro_rules! b_arith {
    ($name:ident, $op:tt) => {
        fn $name(e: &mut Eval) -> bool {
            e.arity(|e, [x, y]| {
                match (x, y) {
                    (Val::Int(l), Val::Int(r)) => {
                        e.push(Val::Int(l $op r));
                        true
                    },
                    (x, y) => {
                        e.stack.extend([y, x]);
                        return false
                    }
                }
            })
        }
    };
}

b_arith!(b_add, +);
b_arith!(b_sub, -);
b_arith!(b_mul, *);
b_arith!(b_div, /);
b_arith!(b_mod, %);

fn b_eq(e: &mut Eval) -> bool {
    e.arity(|e, [x, y]| {
        e.push(if x == y { VAL_TRUE } else { VAL_FALSE });
        true
    })
}

fn b_neq(e: &mut Eval) -> bool {
    e.arity(|e, [x, y]| {
        e.push(if x != y { VAL_TRUE } else { VAL_FALSE });
        true
    })
}

macro_rules! b_cmp {
    ($name:ident, $op:tt) => {
        fn $name(e: &mut Eval) -> bool {
            e.arity(|e, [x, y]| {
                match (x, y) {
                    (Val::Int(l), Val::Int(r)) => {
                        e.push(if l $op r {
                            VAL_TRUE
                        } else {
                            VAL_FALSE
                        });
                        true
                    },
                    (x, y) => {
                        e.stack.extend([y, x]);
                        return false
                    }
                }
            })
        }
    };
}

b_cmp!(b_le, <);
b_cmp!(b_leq, <=);
b_cmp!(b_ge, >);
b_cmp!(b_geq, >=);

fn b_true(e: &mut Eval) -> bool {
    e.push(VAL_TRUE);
    true
}
fn b_false(e: &mut Eval) -> bool {
    e.push(VAL_FALSE);
    true
}

fn b_or(e: &mut Eval) -> bool {
    e.arity(|e, [x, y]| {
        e.push(if x.is_truthy() { x } else { y });
        true
    })
}

fn b_and(e: &mut Eval) -> bool {
    e.arity(|e, [x, y]| {
        e.push(if !x.is_truthy() { x } else { y });
        true
    })
}

fn b_not(e: &mut Eval) -> bool {
    e.arity(|e, [x]| {
        e.push(if x.is_truthy() { VAL_FALSE } else { VAL_TRUE });
        true
    })
}

macro_rules! b_typed {
    ($($name:ident ($e:ident, $($xs:ident),+) if $pred:expr => $body:stmt)+) => {
        $(fn $name(e: &mut Eval) -> bool {
            #[allow(unused_mut)]
            e.arity(|$e, [$(mut $xs),+]| {
                if $pred {
                    $body
                    true
                } else {
                    eprintln!("Type error for `{}`", &stringify!($name).replace('_', "-")[2..]);
                    $e.stack.extend([$($xs),+].into_iter().rev());
                    false
                }
            })
        })+
    };
}

#[inline(always)]
fn index_helper(i0: i64, n0 : usize, allow_end: bool) -> Option<usize> {
    let n = n0 as i64;
    let i = if i0 < 0 {
        if i0 == -1 && n == 0 && allow_end {
            0
        } else {
            i0 + n
        }
    } else {
        i0
    };

    if i < 0 || (i >= n) && !(allow_end && i == n) {
        eprintln!("Index out of bounds: {} but length was {}", i0, n0);
        return None;
    }
    Some(i as usize)
}

b_typed!(
    b_length(e, vs) if vs.is_list() => {
        e.push(Val::Int(vs.len() as i64));
    }

    b_append(e, ls, rs) if ls.is_list() && rs.is_list() => {
        ls.list_mut().extend(rs.into_list().drain(..));
        e.push(ls);
    }

    b_push_back(e, v, vs) if vs.is_list() => {
        vs.list_mut().push_back(v.into());
        e.push(vs);
    }

    b_push_front(e, v, vs) if vs.is_list() => {
        vs.list_mut().push_front(v.into());
        e.push(vs);
    }

    b_head(e, vs) if vs.is_list() => {
        if vs.len() == 0 {
            eprintln!("Can't use `head` on an empty list");
            e.push(vs);
            return false
        }

        e.push(vs.iter().next().unwrap().clone());
    }

    b_tail(e, vs) if vs.is_list() => {
        if vs.len() == 0 {
            eprintln!("Can't use `tail` on an empty list");
            e.push(vs);
            return false
        }
        vs.pop_front();
        e.push(vs);
    }

    b_last(e, vs) if vs.is_list() => {
        if vs.len() == 0 {
            eprintln!("Can't use `head` on an empty list");
            e.push(vs);
            return false
        }

        e.push(vs.iter().last().unwrap().clone());
    }

    b_init(e, vs) if vs.is_list() => {
        if vs.len() == 0 {
            eprintln!("Can't use `tail` on an empty list");
            e.push(vs);
            return false
        }
        vs.pop_back();
        e.push(vs);
    }

    b_head_tail(e, vs) if vs.is_list() => {
        if vs.len() == 0 {
            eprintln!("Can't use `head-tail` on an empty list");
            e.push(vs);
            return false
        }

        let h = vs.pop_front().unwrap().unwrap_or_else(|x| x.clone());
        e.stack.extend([vs, h]);
    }

    b_last_init(e, vs) if vs.is_list() => {
        if vs.len() == 0 {
            eprintln!("Can't use `head-tail` on an empty list");
            e.push(vs);
            return false
        }

        let h = vs.pop_back().unwrap().unwrap_or_else(|x| x.clone());
        e.stack.extend([vs, h.into()]);
    }

    b_nth(e, i0, vs) if i0.is_int() && vs.is_list() => {
        let Some(i) = index_helper(i0.int(), vs.len(), false) else {
            e.stack.extend([vs, i0]);
            return false
        };
        e.push(vs.nth(i).clone());
    }

    b_set_nth(e, i0, v, vs) if i0.is_int() && vs.is_list() => {
        let Some(i) = index_helper(i0.int(), vs.len(), false) else {
            e.stack.extend([vs, v, i0]);
            return false
        };
        vs.list_mut()[i] = v.into();
        e.push(vs);
    }

    b_insert_before(e, i0, v, vs) if i0.is_int() && vs.is_list() => {
        let Some(i) = index_helper(i0.int(), vs.len(), true) else {
            e.stack.extend([vs, v, i0]);
            return false
        };
        vs.list_mut().insert(i, v.into());
        e.push(vs)
    }

    b_remove_nth(e, i0, vs) if i0.is_int() && vs.is_list() => {
        let Some(i) = index_helper(i0.int(), vs.len(), false) else {
            e.stack.extend([vs, i0]);
            return false
        };
        vs.list_mut().remove(i);
        e.push(vs);
    }

    b_swap_remove_nth(e, i0, vs) if i0.is_int() && vs.is_list() => {
        let Some(i) = index_helper(i0.int(), vs.len(), false) else {
            e.stack.extend([vs, i0]);
            return false
        };
        vs.list_mut().swap_remove_back(i);
        e.push(vs);
    }

    b_concat(e, vs) if vs.is_list() && vs.iter().all(|x| x.is_list()) => {
        let size = vs.iter().map(|x| x.len()).sum();
        let mut r = Vals::with_capacity(size);
        r.extend(vs.into_list().into_iter().flat_map(|v| {
            v.into_list()
        }));
        e.push(Val::List(r));
    }

     b_slice(e, from, to, vs) if from.is_int() && to.is_int() && vs.is_list() => {
         let (Some(i), Some(j)) = (index_helper(from.int(), vs.len(), true), index_helper(to.int(), vs.len(), true)) else {
             e.stack.extend([vs, to, from]);
             return false
         };
         if i > j || (i > 0 && vs.len() == 0) {
             eprintln!("Invalid slice range");
             e.stack.extend([vs, to, from]);
             return false
         }
         vs.slice(i, j);
         e.push(vs);
     }
);

fn b_type_of(e: &mut Eval) -> bool {
    e.arity(|e, [x]| {
        match x {
            Val::List(_) => e.push(Val::Kw(LIST_SYM)),
            Val::Int(_) => e.push(Val::Kw(INT_SYM)),
            Val::Sym(_) => e.push(Val::Kw(SYMBOL_SYM)),
            Val::Kw(_) => e.push(Val::Kw(KEYWORD_SYM)),
            Val::Ref(_) => e.push(Val::Kw(LIST_SYM)),
        }
        true
    })
}

macro_rules! b_type_pred {
    ($($name:ident $pred:ident),+) => (
        $(
            fn $name(e: &mut Eval) -> bool {
                e.arity(|e, [x]| {
                    if x.$pred() {
                        e.push(VAL_TRUE)
                    } else {
                        e.push(VAL_FALSE)
                    }
                    true
                })
            }
        )+
    );
}

b_type_pred!(
    b_is_int is_int,
    b_is_symbol is_sym,
    b_is_keyword is_kw,
    b_is_list is_list
);

fn b_quote(e: &mut Eval) -> bool {
    e.arity(|e, [x]| {
        e.push(Val::List(Vals::from([x.into()])));
        true
    })
}

fn b_unquote(e: &mut Eval) -> bool {
    e.arity(|e, [x]| {
        if x.is_list() {
            e.program.extend(x.into_list_ref());
            true
        } else {
            e.push(x);
            return false;
        }
    })
}

fn b_choose(e: &mut Eval) -> bool {
    e.arity(|e, [t, f, c]| {
        if t.is_list() && f.is_list() {
            if c.is_truthy() {
                e.program.extend(t.into_list_ref());
            } else {
                e.program.extend(f.into_list_ref());
            }
            true
        } else {
            e.stack.extend([c, f, t]);
            false
        }
    })
}

fn b_leave_scope(e: &mut Eval) -> bool {
    e.lexicon.pop().is_some()
}

#[inline(always)]
fn scoped_helper<F, G>(e: &mut Eval, fst_cond: F, build_scope: G) -> bool
where
    F: FnOnce(&Val, &Eval) -> bool,
    G: FnOnce(Val, &mut LexiconScope, &mut Eval) -> Option<Val>,
{
    e.arity(|e, [mut bs, v]| {
        'fail: loop {
            if bs.is_list() && v.is_list() && fst_cond(&bs, e) {
                // Tail "call" optimization:
                // Basically in a traditional "tail call" position the last operation is a "return"
                // In our case that's a %{leave-scope}
                // Since there's no residual program between the active scope and the parent scope
                // We can safely just merge the two scopes at scope introduction time and then elide
                // the %{leave-scope}. This even works for non-identical scopes!
                let in_tail_pos = e.program.in_tail_position();
                let mut scope = if in_tail_pos {
                    let Some(s) = e.lexicon.pop() else {
                        eprintln!("Invalid scope!");
                        break 'fail;
                    };
                    s
                } else {
                    LexiconScope::new(bs.len() / 2)
                };
                if let Some(restored_bs) = build_scope(bs, &mut scope, e) {
                    bs = restored_bs;
                    break 'fail;
                }

                e.lexicon.push(scope);
                if !in_tail_pos {
                    e.program.leave_scope();
                }
                e.program.extend(v.into_list_ref());

                return true;
            } else {
                eprintln!("Invalid arguments: locals {:?} {:?}", bs, v);
                break 'fail;
            }
        }
        e.stack.extend([v, bs]);
        false
    })
}

fn b_locals(e: &mut Eval) -> bool {
    scoped_helper(
        e,
        |ls, e| ls.len() <= e.stack.len(),
        |ls, scope, e| {
            for l in ls.iter().rev() {
                let Val::Sym(_) = l else {
                    eprintln!("Invalid local: {:?}", Value(&e.sym_table, l));
                    return Some(ls)
                };
            }

            for (l, v) in ls.iter().rev().zip(e.stack.drain(e.stack.len() - ls.len()..)) {
                scope.insert(l.sym(), Slot::Val(v.into_sharable()));
            }

            None
        },
    )
}
fn b_define(e: &mut Eval) -> bool {
    scoped_helper(
        e,
        |ds, _e| ds.len() % 2 == 0,
        |ds, scope, e| {
            if ds.len() % 2 == 1 {
                eprintln!("Invalid definitions: {{{:?}}}", Value(&e.sym_table, &ds));
                return Some(ds)
            };
            for i in 0..(ds.len() / 2) {
                let k = ds.nth(i * 2);
                let v = ds.nth(i * 2 + 1);
                if !(k.is_sym() && v.is_list()) {
                    eprintln!(
                        "Invalid definition: {:?} {:?}",
                        Value(&e.sym_table, &k),
                        Value(&e.sym_table, &v)
                    );
                    return Some(ds)
                }
                // TODO: We could avoid the clone here
                scope.insert(k.sym(), Slot::Quote(v.as_list_ref()));
            }
            None
        },
    )
}
