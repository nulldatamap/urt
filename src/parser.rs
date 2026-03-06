use std::collections::VecDeque;
use crate::val::*;

pub fn parse(input: &str) -> Result<Vals, String> {
    let mut chars = input.chars().peekable();
    parse_vals(&mut chars, false)
}

fn parse_vals<I>(chars: &mut std::iter::Peekable<I>, stop_on_close: bool) -> Result<Vals, String>
where
    I: Iterator<Item = char>,
{
    let mut vals = VecDeque::new();

    while let Some(&ch) = chars.peek() {
        match ch {
            '{' => {
                chars.next(); // consume '{'
                let inner = parse_vals(chars, true)?;
                vals.push_back(Val::Quote(inner));
            }
            '}' => {
                chars.next(); // consume '}'
                if stop_on_close {
                    return Ok(vals.into());
                } else {
                    return Err("unexpected '}'".into());
                }
            }
            c if c.is_whitespace() => {
                chars.next(); // skip whitespace
            }
            _ => {
                vals.push_back(parse_atom(chars)?);
            }
        }
    }

    if stop_on_close {
        Err("unclosed '{'".into())
    } else {
        Ok(vals.into())
    }
}

fn parse_atom<I>(chars: &mut std::iter::Peekable<I>) -> Result<Val, String>
where
    I: Iterator<Item = char>,
{
    let mut buf = String::new();
    let is_kw = chars.peek() == Some(&':');
    if is_kw {
        chars.next();
    }

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() || ch == '{' || ch == '}' {
            break;
        }
        buf.push(ch);
        chars.next();
    }

    if buf.is_empty() {
        return Err("unexpected token".into());
    }

    let mut chs = buf.chars();
    if is_kw {
        Ok(Val::Kw(buf))
    } else {
        if buf.starts_with('-') && buf.len() > 1 {
            _ = chs.next();
        }

        if chs.all(|c| c.is_ascii_digit()) {
            Ok(Val::Int(buf.parse::<i64>().map_err(|e| {
                let mut x = e.to_string();
                x.push_str(&buf);
                x
            })?))
        } else {
            Ok(Val::Sym(buf))
        }
    }
}
