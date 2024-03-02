use {
    crate::{Argument, Attribute, Class, Function, SelfReference},
    crate::{Error, ErrorKind, MethodError},
    proc_macro::{Delimiter, Span, TokenTree},
    std::iter::Peekable,
};

pub fn parse_function(
    tokens: &mut Peekable<impl Iterator<Item = TokenTree>>,
    start_span: Span,
    current_class: &mut Option<Class>,
    attributes: &[Attribute],
) -> Result<(), Error> {
    let Some(TokenTree::Ident(fn_name)) = tokens.next() else {
        return Err(Error {
            start: start_span,
            end: start_span,
            kind: ErrorKind::Method(MethodError::NoName),
        });
    };
    let Some(TokenTree::Group(fn_args)) = tokens.next() else {
        return Err(Error {
            start: fn_name.span(),
            end: fn_name.span(),
            kind: ErrorKind::Method(MethodError::NoArgs),
        });
    };
    if fn_args.delimiter() != Delimiter::Parenthesis {
        return Err(Error {
            start: fn_name.span(),
            end: fn_name.span(),
            kind: ErrorKind::Method(MethodError::NoArgs),
        });
    }

    let Some(TokenTree::Punct(maybe_semicolon)) = tokens.next() else {
        return Err(Error {
            start: fn_args.span(),
            end: fn_args.span(),
            kind: ErrorKind::Method(MethodError::NoReturnTypeOrSemicolon),
        });
    };
    let return_type = match maybe_semicolon.as_char() {
        ';' => None,
        '-' => {
            let Some(TokenTree::Punct(maybe_arrow)) = tokens.next() else {
                return Err(Error {
                    start: fn_args.span(),
                    end: fn_args.span(),
                    kind: ErrorKind::Method(MethodError::NoReturnTypeOrSemicolon),
                });
            };
            if maybe_arrow.as_char() != '>' {
                return Err(Error {
                    start: fn_args.span(),
                    end: fn_args.span(),
                    kind: ErrorKind::Method(MethodError::NoReturnTypeOrSemicolon),
                });
            }

            let ty = crate::parser::parse_type(tokens, maybe_arrow.span())?;

            let Some(TokenTree::Punct(semicolon)) = tokens.next() else {
                return Err(Error {
                    start: ty.span(),
                    end: ty.span(),
                    kind: ErrorKind::Method(MethodError::NoSemicolon),
                });
            };
            if semicolon.as_char() != ';' {
                return Err(Error {
                    start: ty.span(),
                    end: ty.span(),
                    kind: ErrorKind::Method(MethodError::NoSemicolon),
                });
            }

            Some(ty)
        }
        _ => {
            return Err(Error {
                start: fn_args.span(),
                end: fn_args.span(),
                kind: ErrorKind::Method(MethodError::NoReturnTypeOrSemicolon),
            });
        }
    };

    let Some(ref mut current_class) = current_class else {
        return Err(Error {
            start: start_span,
            end: maybe_semicolon.span(),
            kind: ErrorKind::MethodBeforeClass,
        });
    };

    let (self_reference, args) =
        parse_args(fn_args.stream().into_iter().peekable(), fn_args.span_open())?;

    let mut func = Function {
        name: fn_name.to_string(),
        return_type,
        args,
        self_reference,
        selector: None,
    };

    for attribute in attributes {
        match attribute {
            Attribute::Selector(sel) => func.selector = Some(sel.clone()),
        }
    }

    current_class.methods.push(func);

    Ok(())
}

fn parse_args(
    mut src: Peekable<impl Iterator<Item = TokenTree>>,
    mut last_span: Span,
) -> Result<(SelfReference, Vec<Argument>), Error> {
    let Some(maybe_self) = src.peek() else {
        return Ok((SelfReference::None, Vec::new()));
    };
    let maybe_self = maybe_self.to_string();

    let self_reference = if maybe_self == *"&" {
        let ref_token = src.next().unwrap();
        let Some(TokenTree::Ident(maybe_self)) = src.next() else {
            return Err(Error {
                start: ref_token.span(),
                end: ref_token.span(),
                kind: ErrorKind::Method(MethodError::ExpectedSelfReference),
            });
        };
        match maybe_self.to_string().as_str() {
            "self" => {
                last_span = maybe_self.span();
                SelfReference::Immutable
            }
            "mut" => {
                let Some(TokenTree::Ident(_self)) = src.next() else {
                    return Err(Error {
                        start: ref_token.span(),
                        end: ref_token.span(),
                        kind: ErrorKind::Method(MethodError::ExpectedSelfReference),
                    });
                };
                if _self.to_string() != *"self" {
                    return Err(Error {
                        start: ref_token.span(),
                        end: ref_token.span(),
                        kind: ErrorKind::Method(MethodError::ExpectedSelfReference),
                    });
                }

                last_span = _self.span();
                SelfReference::Mutable
            }
            _ => {
                return Err(Error {
                    start: ref_token.span(),
                    end: ref_token.span(),
                    kind: ErrorKind::Method(MethodError::ExpectedSelfReference),
                })
            }
        }
    } else if maybe_self == *"self" {
        last_span = src.next().unwrap().span();
        SelfReference::Owned
    } else {
        SelfReference::None
    };

    if self_reference != SelfReference::None {
        if let Some(comma) = src.peek() {
            if comma.to_string() != *"," {
                return Err(Error {
                    start: last_span,
                    end: last_span,
                    kind: ErrorKind::Method(MethodError::NoArgumentComma),
                });
            }
            src.next();

            // trailing comma
            if src.peek().is_none() {
                return Ok((self_reference, Vec::new()));
            }
        } else {
            return Ok((self_reference, Vec::new()));
        }
    }

    let mut args = Vec::new();
    loop {
        let Some(TokenTree::Ident(name)) = src.next() else {
            return Err(Error {
                start: last_span,
                end: last_span,
                kind: ErrorKind::Method(MethodError::NoArgumentName),
            });
        };
        let Some(TokenTree::Punct(colon)) = src.next() else {
            return Err(Error {
                start: name.span(),
                end: name.span(),
                kind: ErrorKind::Method(MethodError::NoArgumentColon),
            });
        };
        let ty = crate::parser::parse_type(&mut src, colon.span())?;
        let ty_span = ty.span();

        args.push(Argument {
            name: name.to_string(),
            ty,
        });

        if src.peek().is_some() {
            let Some(TokenTree::Punct(comma)) = src.next() else {
                return Err(Error {
                    start: name.span(),
                    end: ty_span,
                    kind: ErrorKind::Method(MethodError::NoArgumentComma),
                });
            };
            if comma.as_char() != ',' {
                return Err(Error {
                    start: name.span(),
                    end: ty_span,
                    kind: ErrorKind::Method(MethodError::NoArgumentComma),
                });
            }

            // trailing comma
            if src.peek().is_none() {
                break;
            }
        } else {
            break;
        }
    }

    Ok((self_reference, args))
}
