use {
    crate::{Error, ErrorKind, Mutability, Type},
    proc_macro::{Delimiter, Span, TokenTree},
    std::iter::Peekable,
};

pub fn parse_type(
    src: &mut Peekable<impl Iterator<Item = TokenTree>>,
    start_span: Span,
) -> Result<Type, Error> {
    let Some(next) = src.next() else {
        return Err(Error {
            start: start_span,
            end: start_span,
            kind: ErrorKind::NoType,
        });
    };
    match next {
        TokenTree::Ident(ty) => Ok(Type::Absolute(ty.to_string(), ty.span())),
        TokenTree::Punct(punct) => match punct.as_char() {
            '*' => {
                let Some(TokenTree::Ident(const_or_mut)) = src.next() else {
                    return Err(Error {
                        start: punct.span(),
                        end: punct.span(),
                        kind: ErrorKind::GiveUp,
                    });
                };
                let mutability = match const_or_mut.to_string().as_str() {
                    "const" => Mutability::Immut,
                    "mut" => Mutability::Mut,
                    _ => {
                        return Err(Error {
                            start: const_or_mut.span(),
                            end: const_or_mut.span(),
                            kind: ErrorKind::GiveUp,
                        })
                    }
                };
                let other_ty = parse_type(src, const_or_mut.span())?;
                let other_ty_span = other_ty.span();

                Ok(Type::Pointer(mutability, Box::new(other_ty), other_ty_span))
            }
            '&' => {
                // TODO: Figure out safety with borrows and support them.
                Err(Error {
                    start: punct.span(),
                    end: punct.span(),
                    kind: ErrorKind::BorrowsUnsupported,
                })
            }
            _ => Err(Error {
                start: punct.span(),
                end: punct.span(),
                kind: ErrorKind::NoType,
            }),
        },
        TokenTree::Group(group) => {
            if group.delimiter() != Delimiter::Parenthesis {
                return Err(Error {
                    start: group.span_open(),
                    end: group.span_close(),
                    kind: ErrorKind::GiveUp,
                });
            }

            let mut types = Vec::new();
            while src.peek().is_some() {
                types.push(parse_type(src, group.span_open())?);
                if src.peek().is_some() && src.next().unwrap().to_string() != "," {
                    return Err(Error {
                        start: group.span_open(),
                        end: group.span_close(),
                        kind: ErrorKind::NoComma,
                    });
                }
            }

            Ok(Type::Tuple(types, group.span()))
        }
        _ => Err(Error {
            start: next.span(),
            end: next.span(),
            kind: ErrorKind::NoType,
        }),
    }
}
