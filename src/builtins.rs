use crate::eval::{Builtin, Eval};
use crate::val::{Program, VAL_FALSE, VAL_TRUE, Val, Vals};
use std::collections::{HashMap, VecDeque};

pub fn builtins() -> HashMap<&'static str, Builtin> {
    let mut b = HashMap::<&'static str, Builtin>::new();

    // Stack
    b.insert("drop", b_drop);
    b.insert("dup", b_dup);
    b.insert("swap", b_swap);
    // Arithmetic
    b.insert("+", b_add);
    b.insert("-", b_sub);
    b.insert("*", b_mul);
    b.insert("/", b_div);
    b.insert("%", b_mod);
    // Comparison
    b.insert("=", b_eq);
    b.insert("<>", b_neq);
    b.insert("<", b_le);
    b.insert("<=", b_leq);
    b.insert(">", b_ge);
    b.insert(">=", b_geq);
    // Logic
    b.insert("true", b_true);
    b.insert("false", b_false);
    b.insert("&&", b_and);
    b.insert("||", b_or);
    b.insert("not", b_not);
    // Data structure:
    b.insert("length", b_length);
    b.insert("append", b_append);
    b.insert("push-back", b_push_back);
    b.insert("push-front", b_push_front);
    b.insert("head", b_head);
    b.insert("tail", b_tail);
    b.insert("init", b_init);
    b.insert("last", b_last);
    b.insert("head-tail", b_head_tail);
    b.insert("last-init", b_last_init);
    b.insert("nth", b_nth);
    b.insert("set-nth", b_set_nth);
    b.insert("insert-before", b_insert_before);
    b.insert("remove-nth", b_remove_nth);
    b.insert("swap-remove-nth", b_swap_remove_nth);
    b.insert("concat", b_concat);
    b.insert("slice", b_slice);
    // b.insert("build-list", b_build_list);
    // Types
    b.insert("type-of", b_type_of);
    b.insert("int?", b_is_int);
    b.insert("symbol?", b_is_symbol);
    b.insert("keyword?", b_is_keyword);
    b.insert("list?", b_is_list);
    // Quoting
    b.insert("quote", b_quote);
    b.insert("unquote", b_unquote);
    // Control flow
    b.insert("choose", b_choose);
    // Scope
    b.insert("%{leave-scope}", b_leave_scope);
    b.insert("locals", b_locals);
    b.insert("define", b_define);

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
                let [Val::Int(l), Val::Int(r)] = [&x, &y] else {
                    e.stack.extend([y, x]);
                    return false
                };
                e.stack.push(Val::Int(l $op r));
                true
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
        e.stack.push(if x == y { VAL_TRUE } else { VAL_FALSE });
        true
    })
}

fn b_neq(e: &mut Eval) -> bool {
    e.arity(|e, [x, y]| {
        e.stack.push(if x != y { VAL_TRUE } else { VAL_FALSE });
        true
    })
}

macro_rules! b_cmp {
    ($name:ident, $op:tt) => {
        fn $name(e: &mut Eval) -> bool {
            e.arity(|e, [x, y]| {
                let [Val::Int(l), Val::Int(r)] = [&x, &y] else {
                    e.stack.extend([y, x]);
                    return false
                };
                e.stack.push(if l $op r {
                    VAL_TRUE
                } else {
                    VAL_FALSE
                });
                true
            })
        }
    };
}

b_cmp!(b_le, <);
b_cmp!(b_leq, <=);
b_cmp!(b_ge, >);
b_cmp!(b_geq, >=);

fn b_true(e: &mut Eval) -> bool {
    e.stack.push(VAL_TRUE);
    true
}
fn b_false(e: &mut Eval) -> bool {
    e.stack.push(VAL_FALSE);
    true
}

fn b_or(e: &mut Eval) -> bool {
    e.arity(|e, [x, y]| {
        e.stack.push(if x.is_truthy() { x } else { y });
        true
    })
}

fn b_and(e: &mut Eval) -> bool {
    e.arity(|e, [x, y]| {
        e.stack.push(if !x.is_truthy() { x } else { y });
        true
    })
}

fn b_not(e: &mut Eval) -> bool {
    e.arity(|e, [x]| {
        e.stack
            .push(if x.is_truthy() { VAL_FALSE } else { VAL_TRUE });
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
        e.stack.push(Val::Int(vs.len() as i64));
    }

    b_append(e, x : Val::Quote(mut ls), y : Val::Quote(mut rs)) {
        ls.extend(rs.drain(..));
        e.stack.push(Val::Quote(ls));
    }

    b_push_back(e, x : v, y : Val::Quote(mut vs)) {
        vs.push_back(v);
        e.stack.push(Val::Quote(vs));
    }

    b_push_front(e, x : v, y : Val::Quote(mut vs)) {
        vs.push_front(v);
        e.stack.push(Val::Quote(vs));
    }

    b_head(e, x : Val::Quote(mut vs)) {
        if vs.len() == 0 {
            eprintln!("Can't use `head` on an empty list");
            e.stack.push(Val::Quote(vs));
            return false
        }

        e.stack.push(vs.pop_front().unwrap());
    }

    b_tail(e, x : Val::Quote(mut vs)) {
        if vs.len() == 0 {
            eprintln!("Can't use `tail` on an empty list");
            e.stack.push(Val::Quote(vs));
            return false
        }
        _ = vs.pop_front();
        e.stack.push(Val::Quote(vs));
    }

    b_last(e, x : Val::Quote(mut vs)) {
        if vs.len() == 0 {
            eprintln!("Can't use `last` on an empty list");
            e.stack.push(Val::Quote(vs));
            return false
        }

        e.stack.push(vs.pop_back().unwrap());
    }

    b_init(e, x : Val::Quote(mut vs)) {
        if vs.len() == 0 {
            eprintln!("Can't use `init` on an empty list");
            e.stack.push(Val::Quote(vs));
            return false
        }
        _ = vs.pop_back();
        e.stack.push(Val::Quote(vs));
    }

    b_head_tail(e, x : Val::Quote(mut vs)) {
        if vs.len() == 0 {
            eprintln!("Can't use `head-tail` on an empty list");
            e.stack.push(Val::Quote(vs));
            return false
        }

        let h = vs.pop_front().unwrap();
        e.stack.extend([Val::Quote(vs), h]);
    }

    b_last_init(e, x : Val::Quote(mut vs)) {
        if vs.len() == 0 {
            eprintln!("Can't use `last-init` on an empty list");
            e.stack.push(Val::Quote(vs));
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
        e.stack.push(vs.into_iter().nth(i).unwrap());
    }

    b_set_nth(e, x : Val::Int(mut i0), y : v, z : Val::Quote(mut vs)) {
        let Some(i) = index_helper(i0, &vs, false) else {
            e.stack.extend([Val::Quote(vs), v, Val::Int(i0)]);
            return false
        };
        vs[i] = v;
        e.stack.push(Val::Quote(vs));
    }

    b_insert_before(e, x : Val::Int(mut i0), y : v, z : Val::Quote(mut vs)) {
        let Some(i) = index_helper(i0, &vs, true) else {
            e.stack.extend([Val::Quote(vs), v, Val::Int(i0)]);
            return false
        };
        vs.insert(i, v);
        e.stack.push(Val::Quote(vs))
    }

    b_remove_nth(e, x : Val::Int(mut i0), z : Val::Quote(mut vs)) {
        let Some(i) = index_helper(i0, &vs, false) else {
            e.stack.extend([Val::Quote(vs), Val::Int(i0)]);
            return false
        };
        vs.remove(i);
        e.stack.push(Val::Quote(vs))
    }

    b_swap_remove_nth(e, x : Val::Int(mut i0), z : Val::Quote(mut vs)) {
        let Some(i) = index_helper(i0, &vs, false) else {
            e.stack.extend([Val::Quote(vs), Val::Int(i0)]);
            return false
        };
        vs.swap_remove_back(i);
        e.stack.push(Val::Quote(vs))
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
            e.stack.push(Val::Quote(vs));
            return false
        };
        let mut r = Vals::with_capacity(size);
        r.extend(vs.into_iter().flat_map(|v| {
            let Val::Quote(vss) = v else {
                unreachable!();
            };
            vss
        }));
        e.stack.push(Val::Quote(r));
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
        e.stack.push(Val::Quote(vs));
    }
);

fn b_type_of(e: &mut Eval) -> bool {
    e.arity(|e, [x]| {
        match x {
            Val::Quote(_) => e.stack.push(Val::Kw("list".to_string())),
            Val::Int(_) => e.stack.push(Val::Kw("int".to_string())),
            Val::Sym(_) => e.stack.push(Val::Kw("symbol".to_string())),
            Val::Kw(_) => e.stack.push(Val::Kw("keyword".to_string())),
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
                        e.stack.push(VAL_TRUE)
                    } else {
                        e.stack.push(VAL_FALSE)
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
        e.stack.push(Val::Quote(VecDeque::from([x])));
        true
    })
}

fn b_unquote(e: &mut Eval) -> bool {
    e.arity(|e, [x]| {
        let Val::Quote(xs) = x else {
            e.stack.push(x);
            return false;
        };
        e.program.extend(xs.into_iter());
        true
    })
}

fn b_choose(e: &mut Eval) -> bool {
    e.arity(|e, [x, y, z]| {
        if let Val::Quote(t) = &x
            && let Val::Quote(f) = &y
        {
            if z.is_truthy() {
                e.program.extend(t.iter().cloned());
            } else {
                e.program.extend(f.iter().cloned());
            }
            true
        } else {
            e.stack.extend([z, y, x]);
            false
        }
    })
}

fn b_leave_scope(e: &mut Eval) -> bool {
    e.lexicon.pop().is_some()
}

fn scoped_helper<F, G>(e: &mut Eval, fst_cond: F, build_scope: G) -> bool
where
    F: FnOnce(&Vals, &Eval) -> bool,
    G: FnOnce(&Vals, &mut HashMap<String, Vals>, &mut Eval) -> bool,
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
                let in_tail_pos = e.program.back() == Some(&Val::Sym("%{leave-scope}".to_string()));
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
                    e.program.push_back(Val::Sym("%{leave-scope}".to_string()));
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
                scope.insert(l.clone(), VecDeque::from([v]));
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
                scope.insert(k.clone(), v.clone());
            }
            true
        },
    )
}
