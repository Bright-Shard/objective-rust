mod function;
mod parse_type;

pub use parse_type::parse_type;

use {
    crate::{Attribute, AttributeError, Class, Error, ErrorKind},
    proc_macro::{Delimiter, Group, TokenTree},
    std::{collections::hash_map::HashMap, iter::Peekable},
};

pub enum ParserOutput {
    Class(Class),
    RawToken(TokenTree),
}

#[derive(Default)]
struct ClassStore {
    map: HashMap<String, Class>,
}
impl ClassStore {
    pub fn insert(&mut self, class: Class) {
        match self.map.get_mut(class.name.as_str()) {
            Some(old_class) => {
                old_class.methods.extend(class.methods);
            }
            None => {
                let _ = self.map.insert(class.name.clone(), class);
            }
        }
    }

    pub fn into_parser_output(self) -> impl Iterator<Item = ParserOutput> {
        self.map.into_values().map(ParserOutput::Class)
    }
}

pub fn parse_macro_input(
    mut tokens: Peekable<impl Iterator<Item = TokenTree>>,
) -> Result<Vec<ParserOutput>, Error> {
    let mut classes = ClassStore::default();
    let mut output = Vec::new();

    while let Some(raw_token) = tokens.next() {
        let token = raw_token.to_string();

        if token == *"extern"
            && tokens.peek().map(|token| token.to_string().to_lowercase())
                == Some("\"objc\"".into())
        {
            let start_span = raw_token.span();
            tokens.next().unwrap();

            let Some(TokenTree::Group(group)) = tokens.next() else {
                return Err(Error {
                    start: start_span,
                    end: start_span,
                    kind: ErrorKind::UnknownObjcBinding,
                });
            };

            if group.delimiter() != Delimiter::Brace {
                return Err(Error {
                    start: start_span,
                    end: group.span(),
                    kind: ErrorKind::BadBindingBrackets,
                });
            }

            parse_extern_block(group.stream().into_iter().peekable())?
                .into_iter()
                .for_each(|class| {
                    classes.insert(class);
                });
            continue;
        }

        if token == *"mod" {
            if let Some(TokenTree::Ident(_)) = tokens.peek() {
                let mod_name = tokens.next().unwrap();
                let mod_name_span = mod_name.span();

                let mut scope = vec![
                    ParserOutput::RawToken(raw_token),
                    ParserOutput::RawToken(mod_name),
                ];

                let Some(TokenTree::Group(braces)) = tokens.next() else {
                    return Err(Error {
                        start: mod_name_span,
                        end: mod_name_span,
                        kind: ErrorKind::GiveUp,
                    });
                };
                if braces.delimiter() != Delimiter::Brace {
                    return Err(Error {
                        start: mod_name_span,
                        end: mod_name_span,
                        kind: ErrorKind::GiveUp,
                    });
                }

                let scoped_output = parse_macro_input(braces.stream().into_iter().peekable())?;
                let scoped_tokens = crate::codegen::generate(scoped_output)?;
                scope.push(ParserOutput::RawToken(TokenTree::Group(Group::new(
                    Delimiter::Brace,
                    scoped_tokens,
                ))));
                output.extend(scope);

                continue;
            }
        }

        output.push(ParserOutput::RawToken(raw_token));
    }

    output.extend(classes.into_parser_output());
    Ok(output)
}

fn parse_extern_block(
    mut tokens: Peekable<impl Iterator<Item = TokenTree>>,
) -> Result<Vec<Class>, Error> {
    let mut classes = ClassStore::default();
    let mut current_class = None;
    let mut active_attributes = Vec::new();

    while let Some(raw_token) = tokens.next() {
        let token = raw_token.to_string();
        if token == *"type" {
            let Some(TokenTree::Ident(name)) = tokens.next() else {
                return Err(Error {
                    start: raw_token.span(),
                    end: raw_token.span(),
                    kind: ErrorKind::UnnamedClass,
                });
            };
            let Some(TokenTree::Punct(semicolon)) = tokens.next() else {
                return Err(Error {
                    start: raw_token.span(),
                    end: name.span(),
                    kind: ErrorKind::NoSemicolonAfterClass,
                });
            };
            if semicolon.as_char() != ';' {
                return Err(Error {
                    start: raw_token.span(),
                    end: name.span(),
                    kind: ErrorKind::NoSemicolonAfterClass,
                });
            }

            let old_class = current_class.replace(Class::new(name.to_string()));
            if let Some(old) = old_class {
                classes.insert(old);
            }
            active_attributes.clear();
        } else if token == *"fn" {
            function::parse_function(
                &mut tokens,
                raw_token.span(),
                &mut current_class,
                &active_attributes,
            )?;
            active_attributes.clear();
        } else if token == *"#" {
            let Some(TokenTree::Group(brackets)) = tokens.next() else {
                return Err(Error {
                    start: raw_token.span(),
                    end: raw_token.span(),
                    kind: ErrorKind::Attribute(AttributeError::NoBrackets),
                });
            };

            let mut tokens = brackets.stream().into_iter();
            let Some(TokenTree::Ident(name)) = tokens.next() else {
                return Err(Error {
                    start: brackets.span_open(),
                    end: brackets.span_open(),
                    kind: ErrorKind::Attribute(AttributeError::NoName),
                });
            };

            match name.to_string().as_str() {
                "selector" => {
                    let Some(TokenTree::Punct(equals)) = tokens.next() else {
                        return Err(Error {
                            start: name.span(),
                            end: name.span(),
                            kind: ErrorKind::Attribute(AttributeError::NoEquals),
                        });
                    };
                    if equals.as_char() != '=' {
                        return Err(Error {
                            start: equals.span(),
                            end: equals.span(),
                            kind: ErrorKind::Attribute(AttributeError::NoEquals),
                        });
                    }

                    let Some(TokenTree::Literal(selector)) = tokens.next() else {
                        return Err(Error {
                            start: equals.span(),
                            end: equals.span(),
                            kind: ErrorKind::Attribute(AttributeError::NoValue),
                        });
                    };
                    let selector_name = selector.to_string();
                    if selector_name.as_bytes()[0] != b'"'
                        || selector_name.as_bytes()[selector_name.len() - 1] != b'"'
                    {
                        return Err(Error {
                            start: selector.span(),
                            end: selector.span(),
                            kind: ErrorKind::Attribute(AttributeError::Type("String".into())),
                        });
                    }

                    active_attributes.push(Attribute::Selector(
                        selector_name[1..selector_name.len() - 1].into(),
                    ));
                }
                _ => {
                    return Err(Error {
                        start: name.span(),
                        end: name.span(),
                        kind: ErrorKind::Attribute(AttributeError::Unknown),
                    });
                }
            }
        }
    }
    if let Some(current) = current_class {
        classes.insert(current);
    }

    Ok(classes.map.into_values().collect())
}
