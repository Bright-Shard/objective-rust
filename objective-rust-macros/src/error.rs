use std::fmt::Display;

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

pub struct Error {
    pub start: Span,
    pub end: Span,
    pub kind: ErrorKind,
}

pub enum ErrorKind {
    /// `extern "objc"` was used beside something that wasn't a block.
    UnknownObjcBinding,
    /// `extern "objc"` blocks must use {} brackets.
    BadBindingBrackets,
    /// A method was defined before the class it is present for.
    MethodBeforeClass,
    /// No name was defined after a `type` keyword.
    UnnamedClass,
    /// There was no `;` after a class name.
    NoSemicolonAfterClass,
    /// A class was defined twice. Stores the class name.
    ClassDefinedTwice(String),
    /// A type was expected but not found.
    NoType,
    /// &T/&mut T are currently unsupported
    BorrowsUnsupported,
    /// An error while parsing a method.
    Method(MethodError),
    /// An error while parsing an attribute macro.
    Attribute(AttributeError),
    /// The parser gave up, it probably found invalid Rust syntax.
    GiveUp,
    /// Expected a comma between types
    NoComma,
}
impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = match self {
            Self::UnknownObjcBinding => {
                "Unknown Objective-C binding. Objective-rust only supports `extern \"objc\"` blocks.".into()
            }
            Self::BadBindingBrackets => "`extern \"objc\"` blocks must use `{}`.".into(),
            Self::MethodBeforeClass => "A class needs to be defined before methods can be defined.".into(),
            Self::UnnamedClass => "Expected a class name after `type`.".into(),
            Self::NoSemicolonAfterClass => "Expected a `;` beside the class name.".into(),
            Self::ClassDefinedTwice(name) => format!("Class {name} is defined multiple times."),
            Self::NoType => "Expected a type here.".into(),
            Self::BorrowsUnsupported => "Borrows are currently unsupported in Objective-Rust for safety reasons.".into(),
            Self::Method(method) => method.to_string(),
            Self::Attribute(err) => err.to_string(),
            Self::GiveUp => "Unknown syntax".into(),
            Self::NoComma => "Expected a comma between types".into(),
        };
        write!(f, "{err}")
    }
}

/// Errors while parsing a method definition.
pub enum MethodError {
    /// There was no name after the `fn` definition.
    NoName,
    /// There were no arguments after the method name.
    NoArgs,
    /// There was no return type or `;` after the method arguments.
    NoReturnTypeOrSemicolon,
    /// There was no `;` after a return type.
    NoSemicolon,
    /// There was no name for a method argument.
    NoArgumentName,
    /// There was no `:` after a method argument's name.
    NoArgumentColon,
    /// There was no comma in between method arguments.
    NoArgumentComma,
    /// Found an `&`, but no `self` or `mut self` after it, in method arguments.
    ExpectedSelfReference,
}
impl Display for MethodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = match self {
            Self::NoName => "Expected a method name after `fn`.",
            Self::NoArgs => "Expected method arguments after the method name.",
            Self::NoReturnTypeOrSemicolon => {
                "Expected a return type or `;` after the method arguments."
            }
            Self::NoSemicolon => "Expected a `;` after the method return type.",
            Self::NoArgumentName => "Expected an argument name.",
            Self::NoArgumentColon => "Expected a `:` after the argument's name.",
            Self::NoArgumentComma => "Expected a `,` in between arguments.",
            Self::ExpectedSelfReference => "Expected `self` or `mut self` after the `&`.",
        };
        write!(f, "{err}")
    }
}

pub enum AttributeError {
    /// No brackets in an attribute (like `#[selector]`).
    NoBrackets,
    /// No name was given for the attribute.
    NoName,
    /// An unknown name was given for the attribute.
    Unknown,
    /// No `=` was found after the attribute name.
    NoEquals,
    /// No value was found after a `=` in an attribute assignment.
    NoValue,
    /// An unexpected type was used for the attribute's value.
    /// Stores the expected type.
    Type(String),
}
impl Display for AttributeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = match self {
            Self::NoBrackets => "Expected brackets afer `#` in attribute.".into(),
            Self::NoName => "Expected an attribute name after `[`.".into(),
            Self::Unknown => "Unknown attribute.".into(),
            Self::NoEquals => "Expected `=` after the attribute name.".into(),
            Self::NoValue => "Expected a value after the `=`.".into(),
            Self::Type(expected) => format!("Expected a `{expected}` literal."),
        };
        write!(f, "{err}")
    }
}

impl From<Error> for TokenStream {
    fn from(value: Error) -> Self {
        TokenStream::from_iter(vec![
            TokenTree::Punct({
                let mut punct = Punct::new(':', Spacing::Joint);
                punct.set_span(value.start);
                punct
            }),
            TokenTree::Punct({
                let mut punct = Punct::new(':', Spacing::Alone);
                punct.set_span(value.start);
                punct
            }),
            TokenTree::Ident(Ident::new("core", value.start)),
            TokenTree::Punct(Punct::new(':', Spacing::Joint)),
            TokenTree::Punct(Punct::new(':', Spacing::Alone)),
            TokenTree::Ident(Ident::new("compile_error", value.start)),
            TokenTree::Punct(Punct::new('!', Spacing::Alone)),
            TokenTree::Group({
                let mut group = Group::new(
                    Delimiter::Brace,
                    TokenStream::from_iter(vec![TokenTree::Literal(Literal::string(
                        &value.kind.to_string(),
                    ))]),
                );
                group.set_span(value.end);
                group
            }),
        ])
    }
}
