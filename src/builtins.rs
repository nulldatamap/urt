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
    /*
    b.insert("push-back", b_push_back);
    b.insert("push-front", b_push_front);
    b.insert("head", b_head);
    b.insert("tail", b_tail);
    b.insert("init", b_init);
    b.insert("last", b_last);
    b.insert("nth", b_nth);
    b.insert("set-nth", b_set_nth);
    // b.insert("insert", b_insert);
    // b.insert("remove-nth", b_remove_nth);
    // b.insert("swap-remove-nth", b_swap_remove_nth);
    // b.insert("concat", b_concat);
    */
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
    ($name:ident ($e:ident, $($xs:ident : $tys:pat),+) $body:stmt) => {
        fn $name(e: &mut Eval) -> bool {
            e.arity(|$e, [$($xs),+]| {
                if $(let $tys = &$xs)&&+ {
                    $body
                    true
                } else {
                    eprintln!("Type error for `{}`", stringify!($name));
                    $e.stack.extend([$($xs),+].into_iter().rev());
                    false
                }
            })
        }
    };
}

b_typed!(
    b_length(e, x : Val::Quote(vs)) {
        e.stack.push(Val::Int(vs.len() as i64));
    }
);

b_typed!(
    b_append(e, x : Val::Quote(ls), y : Val::Quote(rs)) {
        let mut new = Vals::with_capacity(ls.len() + rs.len());
        new.extend(ls.iter().cloned());
        new.extend(rs.iter().cloned());
        e.stack.push(Val::Quote(new));
    }
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
