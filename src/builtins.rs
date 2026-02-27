use crate::eval::{Builtin, Eval};
use crate::val::Val;
use std::collections::HashMap;

pub fn builtins() -> HashMap<&'static str, Builtin> {
    let mut b = HashMap::<&'static str, Builtin>::new();

    b.insert("drop", b_drop);
    b.insert("dup", b_dup);
    b.insert("swap", b_swap);
    b.insert("quote", b_quote);
    b.insert("unquote", b_unquote);
    b.insert("choose", b_choose);
    b.insert("%{leave-scope}", b_leave_scope);
    b.insert("locals", b_locals);
    b.insert("define", b_define);

    b
}

fn b_drop(e: &mut Eval) -> bool {
    e.arity(|e, [_]| true)
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

fn b_quote(e: &mut Eval) -> bool {
    e.arity(|e, [x]| {
        e.stack.push(Val::Quote(vec![x]));
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
            e.program.extend([x, y, z]);
            false
        }
    })
}

fn b_leave_scope(e: &mut Eval) -> bool {
    e.lexicon.pop().is_some()
}

fn b_locals(e: &mut Eval) -> bool {
    e.arity(|e, [x, y]| {
        'fail: loop {
            if let Val::Quote(ls) = &x
                && let Val::Quote(v) = &y
                && ls.len() <= e.stack.len()
            {
                let mut scope = HashMap::new();
                let mut lss = vec![];
                for l in ls.iter().rev() {
                    let Val::Sym(l) = l else {
                        eprintln!("Invalid local: {:?}", l);
                        break 'fail;
                    };
                    lss.push(l.clone());
                }

                for (l, v) in lss.iter().zip(e.stack.drain(e.stack.len() - ls.len()..)) {
                    scope.insert(l.clone(), vec![v]);
                }

                e.lexicon.push(scope);
                e.program.push(Val::Sym("%{leave-scope}".to_string()));
                e.program.extend(v.clone());

                return true;
            } else {
                eprintln!("Invalid arguments: locals {:?} {:?}", x, y);
                break 'fail;
            }
        }
        e.program.extend([x, y]);
        false
    })
}
fn b_define(e: &mut Eval) -> bool {
    e.arity(|e, [x, y]| {
        'fail: loop {
            if let Val::Quote(ds) = &x
                && let Val::Quote(v) = &y
                && ds.len() % 2 == 0
            {
                let mut scope = HashMap::new();
                let (kvs, []) = ds.as_chunks() else {
                    eprintln!("Invalid definitions: {:?}", x);
                    break 'fail;
                };
                for kv in kvs {
                    let [Val::Sym(k), Val::Quote(v)] = kv else {
                        eprintln!("Invalid definition: {:?} {:?}", &kv[0], &kv[1]);
                        break 'fail;
                    };
                    scope.insert(k.clone(), v.clone());
                }

                e.lexicon.push(scope);
                e.program.push(Val::Sym("%{leave-scope}".to_string()));
                e.program.extend(v.clone());

                return true;
            } else {
                eprintln!("Invalid arguments: define {:?} {:?}", x, y);
                break 'fail;
            }
        }
        e.program.extend([x, y]);
        false
    })
}
