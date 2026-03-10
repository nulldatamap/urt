use crate::eval::{Builtin, Eval, Ref, ValOrRef};
use crate::val::{Program, VAL_FALSE, VAL_TRUE, Val, Vals, SymbolTable, Sym, LIST_SYM, KEYWORD_SYM, SYMBOL_SYM, INT_SYM, LEAVE_SCOPE_SYM};
use std::collections::{HashMap, VecDeque};

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
                    (ValOrRef::Val(Val::Int(l)), ValOrRef::Val(Val::Int(r))) => {
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
                    (ValOrRef::Val(Val::Int(l)), ValOrRef::Val(Val::Int(r))) => {
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
    ($($name:ident ($e:ident, $($xs:ident : $tys:pat),+) $body:stmt)+) => {
        $(fn $name(e: &mut Eval) -> bool {
            e.arity(|$e, [$($xs),+]| {
                match [$($xs),+] {
                    [$($tys),+]=> {
                        $body
                        true
                    },
                    xs => {
                        eprintln!("Type error for `{}`", &stringify!($name).replace('_', "-")[2..]);
                        $e.stack.extend(xs.into_iter().rev());
                        false
                    }
                }
            })
        })+
    };
}

#[inline(always)]
fn index_helper(i0: i64, vs: &Vals, allow_end: bool) -> Option<usize> {
    let n = vs.len() as i64;
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
        eprintln!("Index out of bounds: {} but length was {}", i0, vs.len());
        return None;
    }
    Some(i as usize)
}

b_typed!(
    b_length(e, x : Val::Quote(vs)) {
        e.push(Val::Int(vs.len() as i64));
    }

    b_append(e, x : Val::Quote(mut ls), y : Val::Quote(mut rs)) {
        ls.extend(rs.drain(..));
        e.push(Val::Quote(ls));
    }

    b_push_back(e, x : v, y : Val::Quote(mut vs)) {
        vs.push_back(v);
        e.push(Val::Quote(vs));
    }

    b_push_front(e, x : v, y : Val::Quote(mut vs)) {
        vs.push_front(v);
        e.push(Val::Quote(vs));
    }

    b_head(e, x : Val::Quote(mut vs)) {
        if vs.len() == 0 {
            eprintln!("Can't use `head` on an empty list");
            e.push(Val::Quote(vs));
            return false
        }

        e.push(vs.pop_front().unwrap());
    }

    b_tail(e, x : Val::Quote(mut vs)) {
        if vs.len() == 0 {
            eprintln!("Can't use `tail` on an empty list");
            e.push(Val::Quote(vs));
            return false
        }
        _ = vs.pop_front();
        e.push(Val::Quote(vs));
    }

    b_last(e, x : Val::Quote(mut vs)) {
        if vs.len() == 0 {
            eprintln!("Can't use `last` on an empty list");
            e.push(Val::Quote(vs));
            return false
        }

        e.push(vs.pop_back().unwrap());
    }

    b_init(e, x : Val::Quote(mut vs)) {
        if vs.len() == 0 {
            eprintln!("Can't use `init` on an empty list");
            e.push(Val::Quote(vs));
            return false
        }
        _ = vs.pop_back();
        e.push(Val::Quote(vs));
    }

    b_head_tail(e, x : Val::Quote(mut vs)) {
        if vs.len() == 0 {
            eprintln!("Can't use `head-tail` on an empty list");
            e.push(Val::Quote(vs));
            return false
        }

        let h = vs.pop_front().unwrap();
        e.stack.extend([Val::Quote(vs), h]);
    }

    b_last_init(e, x : Val::Quote(mut vs)) {
        if vs.len() == 0 {
            eprintln!("Can't use `last-init` on an empty list");
            e.push(Val::Quote(vs));
            return false
        }

        let h = vs.pop_back().unwrap();
        e.stack.extend([Val::Quote(vs), h]);
    }

    b_nth(e, x : Val::Int(mut i0), y : Val::Quote(mut vs)) {
        let Some(i) = index_helper(i0, &vs, false) else {
            e.stack.extend([Val::Quote(vs), Val::Int(i0)]);
            return false
        };
        e.push(vs.into_iter().nth(i).unwrap());
    }

    b_set_nth(e, x : Val::Int(mut i0), y : v, z : Val::Quote(mut vs)) {
        let Some(i) = index_helper(i0, &vs, false) else {
            e.stack.extend([Val::Quote(vs), v, Val::Int(i0)]);
            return false
        };
        vs[i] = v;
        e.push(Val::Quote(vs));
    }

    b_insert_before(e, x : Val::Int(mut i0), y : v, z : Val::Quote(mut vs)) {
        let Some(i) = index_helper(i0, &vs, true) else {
            e.stack.extend([Val::Quote(vs), v, Val::Int(i0)]);
            return false
        };
        vs.insert(i, v);
        e.push(Val::Quote(vs))
    }

    b_remove_nth(e, x : Val::Int(mut i0), z : Val::Quote(mut vs)) {
        let Some(i) = index_helper(i0, &vs, false) else {
            e.stack.extend([Val::Quote(vs), Val::Int(i0)]);
            return false
        };
        vs.remove(i);
        e.push(Val::Quote(vs))
    }

    b_swap_remove_nth(e, x : Val::Int(mut i0), z : Val::Quote(mut vs)) {
        let Some(i) = index_helper(i0, &vs, false) else {
            e.stack.extend([Val::Quote(vs), Val::Int(i0)]);
            return false
        };
        vs.swap_remove_back(i);
        e.push(Val::Quote(vs))
    }

    b_concat(e, x : Val::Quote(mut vs)) {
        let Some(size) = vs.iter().try_fold(0, |s, v| {
            if let Val::Quote(vss) = v {
                Some(s + vss.len())
            } else {
                None
            }
        }) else {
            eprintln!("Type error for `concat`");
            e.push(Val::Quote(vs));
            return false
        };
        let mut r = Vals::with_capacity(size);
        r.extend(vs.into_iter().flat_map(|v| {
            let Val::Quote(vss) = v else {
                unreachable!();
            };
            vss
        }));
        e.push(Val::Quote(r));
    }

    b_slice(e, x : Val::Int(from), y : Val::Int(to), z : Val::Quote(mut vs)) {
        let (Some(i), Some(j)) = (index_helper(from, &vs, true), index_helper(to, &vs, true)) else {
            e.stack.extend([Val::Quote(vs), Val::Int(to), Val::Int(from)]);
            return false
        };
        if i > j || (i > 0 && vs.len() == 0) {
            eprintln!("Invalid slice range");
            e.stack.extend([Val::Quote(vs), Val::Int(to), Val::Int(from)]);
            return false
        }
        _ = vs.drain(j..);
        _ = vs.drain(..i);
        e.push(Val::Quote(vs));
    }
);

fn b_type_of(e: &mut Eval) -> bool {
    e.arity(|e, [x]| {
        match x {
            ValOrRef::Val(Val::Quote(_)) => e.push(Val::Kw(LIST_SYM)),
            ValOrRef::Val(Val::Int(_)) => e.push(Val::Kw(INT_SYM)),
            ValOrRef::Val(Val::Sym(_)) => e.push(Val::Kw(SYMBOL_SYM)),
            ValOrRef::Val(Val::Kw(_)) => e.push(Val::Kw(KEYWORD_SYM)),
            ValOrRef::Ref(_) => e.push(Val::Kw(LIST_SYM)),
        }
        true
    })
}

macro_rules! b_type_pred {
    ($($name:ident $ty:pat),+) => (
        $(
            fn $name(e: &mut Eval) -> bool {
                e.arity(|e, [x]| {
                    if let $ty = x {
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
    b_is_int Val::Int(_),
    b_is_symbol Val::Sym(_),
    b_is_keyword Val::Kw(_),
    b_is_list Val::Quote(_)
);

fn b_quote(e: &mut Eval) -> bool {
    e.arity(|e, [x]| {
        e.push(Val::Quote(Vals::from([x.into()])));
        true
    })
}

fn b_unquote(e: &mut Eval) -> bool {
    e.arity(|e, [x]| {
        let Val::Quote(xs) = x else {
            e.push(x);
            return false;
        };
        e.program.extend(xs.into_iter());
        true
    })
}

fn b_choose(e: &mut Eval) -> bool {
    e.arity(|e, [x, y, z]| match (x, y) {
        (Val::Quote(t), Val::Quote(f)) => {
            if z.is_truthy() {
                e.program.extend(t);
            } else {
                e.program.extend(f);
            }
            true
        }
        (x, y) => {
            e.stack.extend([z, y, x]);
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
    F: FnOnce(&Vals, &Eval) -> bool,
    G: FnOnce(&Vals, &mut HashMap<Sym, Ref>, &mut Eval) -> bool,
{
    e.arity(|e, [x, y]| {
        'fail: loop {
            if let Val::Quote(ls) = &x
                && let Val::Quote(v) = &y
                && fst_cond(ls, e)
            {
                // Tail "call" optimization:
                // Basically in a traditional "tail call" position the last operation is a "return"
                // In our case that's a %{leave-scope}
                // Since there's no residual program between the active scope and the parent scope
                // We can safely just merge the two scopes at scope introduction time and then elide
                // the %{leave-scope}. This even works for non-identical scopes!
                let in_tail_pos = e.program.back() == Some(&Val::Sym(LEAVE_SCOPE_SYM));
                let mut scope = if in_tail_pos {
                    let Some(s) = e.lexicon.pop() else {
                        eprintln!("Invalid scope!");
                        break 'fail;
                    };
                    s
                } else {
                    HashMap::new()
                };
                if !build_scope(ls, &mut scope, e) {
                    break 'fail;
                }

                e.lexicon.push(scope);
                if !in_tail_pos {
                    e.program.push_back(Val::Sym(LEAVE_SCOPE_SYM));
                }
                e.program.extend(v.clone());

                return true;
            } else {
                eprintln!("Invalid arguments: locals {:?} {:?}", x, y);
                break 'fail;
            }
        }
        e.stack.extend([y, x]);
        false
    })
}

fn b_locals(e: &mut Eval) -> bool {
    scoped_helper(
        e,
        |ls, e| ls.len() <= e.stack.len(),
        |ls, scope, e| {
            let mut lss = vec![];
            for l in ls.iter().rev() {
                let Val::Sym(l) = l else {
                    eprintln!("Invalid local: {:?}", l);
                    return false;
                };
                lss.push(l.clone());
            }

            for (l, v) in lss.iter().zip(e.stack.drain(e.stack.len() - ls.len()..)) {
                scope.insert(*l, Ref::new(Vals::from([v.into()])));
            }

            true
        },
    )
}
fn b_define(e: &mut Eval) -> bool {
    scoped_helper(
        e,
        |ds, _e| ds.len() % 2 == 0,
        |ds, scope, _e| {
            if ds.len() % 2 == 1 {
                eprintln!("Invalid definitions: {{{:?}}}", Program(ds));
                return false;
            };
            for i in 0..(ds.len() / 2) {
                let kv = [&ds[i * 2], &ds[i * 2 + 1]];
                let [Val::Sym(k), Val::Quote(v)] = kv else {
                    eprintln!("Invalid definition: {:?} {:?}", &kv[0], &kv[1]);
                    return false;
                };
                scope.insert(*k, Ref::new(v.clone()));
            }
            true
        },
    )
}
