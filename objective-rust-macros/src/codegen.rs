use {
    crate::{
        parser::ParserOutput, Argument, Class, Error, Function, Mutability, SelfReference, Type,
    },
    proc_macro::TokenStream,
    std::fmt::Display,
};

pub fn generate(parser_output: Vec<ParserOutput>) -> Result<TokenStream, Error> {
    let mut result = TokenStream::new();

    for output in parser_output {
        match output {
            ParserOutput::Class(class) => {
                result.extend([class.to_string().parse::<TokenStream>().unwrap()])
            }
            ParserOutput::RawToken(token) => result.extend([token]),
        }
    }

    Ok(result)
}

impl Display for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let struct_name = &self.name;
        let mut struct_fields = String::new();
        let mut constructor = String::new();
        let mut struct_fns = String::new();

        for method in &self.methods {
            let Function {
                name,
                return_type,
                args,
                self_reference,
                selector,
            } = method;
            let selector = selector.as_ref().unwrap_or(name);

            if return_type.is_some() {
                let Some(Type::Pointer(_, _, _)) = return_type else {
                    panic!("Only pointer returns are currently supported");
                };
            }

            let mut args_with_types = String::new();
            let mut args_no_types = String::new();
            for arg in args {
                let Argument { name, ty } = arg;
                args_with_types += &format!(", {name}: {ty}");
                args_no_types += &format!(", {name}");
            }

            let return_type_formatted = if let Some(ret) = return_type {
                format!("-> {ret}").replace("Self", struct_name)
            } else {
                String::new()
            };

            let fn_args = if *self_reference == SelfReference::None && args_with_types.len() > 2 {
                &args_with_types[2..]
            } else {
                args_with_types.as_str()
            };
            struct_fns +=
                &format!("pub fn {name}({self_reference}{fn_args}){return_type_formatted}",);

            let instance_ty = match self_reference {
                SelfReference::None => "objective_rust::ffi::Class",
                SelfReference::Mutable => "*mut ()",
                SelfReference::Immutable => "*const ()",
                SelfReference::Owned => panic!("Methods must take `&self` or `&mut self`"),
            };

            let c_fn = format!(
                "extern \"C\" fn(instance: {instance_ty}, sel: objective_rust::ffi::Selector{args_with_types}){return_type_formatted}"
            );

            match self_reference {
                SelfReference::None => {
                    struct_fns += &format!(
                        r#"
                        {{
                            use {{
                                std::{{mem, cell::{{RefCell, OnceCell}}}},
                                objective_rust::ffi::{{self, Selector}}
                            }};

                            thread_local! {{
                                static FUNC: ({c_fn}, Selector) = {{
                                    let meta_class = {struct_name}::get_objc_metaclass();
                                    let name = ffi::get_selector("{selector}").unwrap();
                                    let implementation = ffi::get_method_impl(meta_class, name);
                                    let method = unsafe {{ mem::transmute(implementation) }};

                                    (method, name)
                                }};
                            }}

                            FUNC.with(|(func, sel)| func(Self::get_objc_class(), *sel{args_no_types}))
                        }}
                        "#
                    );
                }
                SelfReference::Mutable | SelfReference::Immutable => {
                    struct_fields +=
                        &format!("{name}_ptr: ({c_fn}, objective_rust::ffi::Selector),");
                    let get_sel =
                        format!("objective_rust::ffi::get_selector(\"{selector}\").unwrap()");
                    let get_impl = format!(
                        "objective_rust::ffi::get_method_impl(Self::get_objc_class(), {get_sel}).unwrap()"
                    );
                    constructor += &format!(
                        "{name}_ptr: (unsafe {{ core::mem::transmute({get_impl}) }}, {get_sel}),"
                    );
                    struct_fns += &format!(
                        "
                        {{
                            (self.{name}_ptr.0)(self.instance.as_ptr(), self.{name}_ptr.1{args_no_types})
                        }}
                        "
                    );
                }
                SelfReference::Owned => unreachable!(),
            }
        }

        write!(
            f,
            r#"
            pub struct {struct_name} {{
                instance: std::ptr::NonNull<()>,
                {struct_fields}
            }}
            impl {struct_name} {{
                /// Attempts to create a new `{struct_name}` from a pointer. Fails if the
                /// pointer is null.
                ///
                /// # Safety
                /// - The pointer must point to a valid instance of the class `{struct_name}` represents.
                /// - The pointer must be valid for as long as `Self` lives.
                pub unsafe fn from_raw(ptr: *mut Self) -> Option<Self> {{
                    let instance = std::ptr::NonNull::new(ptr.cast())?;

                    Some(Self {{
                        instance,
                        {constructor}
                    }})
                }}

                /// Returns the Objective-C class this struct binds to.
                pub fn get_objc_class() -> objective_rust::ffi::Class {{
                    use {{
                        std::{{cell::OnceCell, ptr::addr_of}},
                        objective_rust::ffi::{{self, Class}}
                    }};

                    thread_local! {{
                        static PTR: Class = ffi::get_class("{struct_name}").unwrap();
                    }}

                    PTR.with(|ptr| ptr.clone())
                }}

                /// Returns thie Objective-C metaclass for the class this struct binds to.
                pub fn get_objc_metaclass() -> objective_rust::ffi::Class {{
                    use {{
                        std::{{cell::OnceCell, ptr::addr_of}},
                        objective_rust::ffi::{{self, Class}}
                    }};

                    thread_local! {{
                        static PTR: Class = ffi::get_metaclass("{struct_name}").unwrap();
                    }}

                    PTR.with(|ptr| ptr.clone())
                }}

                {struct_fns}
            }}
            "#,
        )
    }
}

impl Display for SelfReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::None => "",
            Self::Owned => "self",
            Self::Immutable => "&self",
            Self::Mutable => "&mut self",
        };
        write!(f, "{text}")
    }
}
impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Absolute(ty, _) => ty.clone(),
            Self::Borrow(mutability, ty, _) => match mutability {
                Mutability::Immut => format!("&{ty}"),
                Mutability::Mut => format!("&mut {ty}"),
            },
            Self::Pointer(mutability, ty, _) => match mutability {
                Mutability::Immut => format!("*const {ty}"),
                Mutability::Mut => format!("*mut {ty}"),
            },
            Self::Tuple(types, _) => {
                let mut text = "(".to_string();
                for ty in types {
                    text += &ty.to_string();
                    text += ","
                }
                text += ")";

                text
            }
        };

        write!(f, "{text}")
    }
}
