mod codegen;
mod error;
mod parser;

use {
    error::*,
    proc_macro::{Span, TokenStream},
};

#[proc_macro_attribute]
pub fn objrs(_: TokenStream, src: TokenStream) -> TokenStream {
    let tokens = src.into_iter().peekable();

    match parser::parse_macro_input(tokens) {
        Ok(output) => match codegen::generate(output) {
            Ok(result) => result,
            Err(err) => err.into(),
        },
        Err(err) => err.into(),
    }
}

struct Class {
    name: String,
    methods: Vec<Function>,
}
impl Class {
    pub fn new(name: String) -> Self {
        Self {
            name,
            methods: Vec::new(),
        }
    }
}
struct Function {
    name: String,
    return_type: Option<Type>,
    args: Vec<Argument>,
    self_reference: SelfReference,
    selector: Option<String>,
}
struct Argument {
    name: String,
    ty: Type,
}
enum Type {
    Pointer(Mutability, Box<Self>, Span),
    #[allow(dead_code)] // TODO: Support borrows. Need to think through safety.
    Borrow(Mutability, Box<Self>, Span),
    Absolute(String, Span),
    Tuple(Vec<Self>, Span),
}
impl Type {
    pub fn span(&self) -> Span {
        match self {
            Self::Pointer(_, _, span) => *span,
            Self::Borrow(_, _, span) => *span,
            Self::Absolute(_, span) => *span,
            Self::Tuple(_, span) => *span,
        }
    }
}
enum Mutability {
    Mut,
    Immut,
}
#[derive(PartialEq)]
enum SelfReference {
    /// Static/class method
    None,
    /// &self
    Immutable,
    /// &mut self
    Mutable,
    /// self
    Owned,
}
enum Attribute {
    /// Sets the name objective-rust will use to find a method's selector.
    Selector(String),
}
