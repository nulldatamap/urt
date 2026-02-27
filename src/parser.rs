use crate::val::*;

pub fn parse(input: &str) -> Result<Vals, String> {
    let mut chars = input.chars().peekable();
    parse_vals(&mut chars, false)
}

fn parse_vals<I>(chars: &mut std::iter::Peekable<I>, stop_on_close: bool) -> Result<Vals, String>
where
    I: Iterator<Item = char>,
{
    let mut vals = Vec::new();

    while let Some(&ch) = chars.peek() {
        match ch {
            '{' => {
                chars.next(); // consume '{'
                let inner = parse_vals(chars, true)?;
                vals.push(Val::Quote(inner));
            }
            '}' => {
                chars.next(); // consume '}'
                if stop_on_close {
                    return Ok(vals);
                } else {
                    return Err("unexpected '}'".into());
                }
            }
            c if c.is_whitespace() => {
                chars.next(); // skip whitespace
            }
            _ => {
                vals.push(parse_atom(chars)?);
            }
        }
    }

    if stop_on_close {
        Err("unclosed '{'".into())
    } else {
        Ok(vals)
    }
}

fn parse_atom<I>(chars: &mut std::iter::Peekable<I>) -> Result<Val, String>
where
    I: Iterator<Item = char>,
{
    let mut buf = String::new();

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

    if buf.chars().all(|c| c.is_ascii_digit()) {
        Ok(Val::Int(buf.parse::<i64>().map_err(|e| e.to_string())?))
    } else {
        Ok(Val::Sym(buf))
    }
}
