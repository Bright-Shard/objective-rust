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
        let class_name = &self.name;
        let mut struct_fns = String::new();
        let mut vtable_entries = String::new();
        let mut vtable_setup = String::new();
        let mut vtable_constructor = String::new();

        for method in &self.methods {
            let Function {
                name,
                return_type,
                args,
                self_reference,
                selector,
            } = method;
            let selector = selector.as_ref().unwrap_or(name);

            let mut args_with_types = String::new();
            let mut args_no_types = String::new();
            for arg in args {
                let Argument { name, ty } = arg;
                args_with_types += &format!(", {name}: {ty}");
                args_no_types += &format!(", {name}");
            }

            let return_type_formatted = if let Some(ret) = return_type {
                format!("-> {ret}").replace("Self", &format!("{class_name}Instance"))
            } else {
                String::new()
            };

            let instance_ty = match self_reference {
                SelfReference::None => "objective_rust::ffi::Class".into(),
                SelfReference::Mutable => format!("*mut {class_name}Instance"),
                SelfReference::Immutable => format!("*const {class_name}Instance"),
                SelfReference::Owned => panic!("Methods must take `&self` or `&mut self`"),
            };

            let c_fn = format!(
                "
                extern \"C\" fn(
                    instance: {instance_ty},
                    sel: objective_rust::ffi::Selector
                    {args_with_types}
                ){return_type_formatted}
                "
            );

            let class = match self_reference {
                SelfReference::None => "metaclass",
                SelfReference::Mutable | SelfReference::Immutable => "class",
                SelfReference::Owned => panic!("Objective-C methods cannot own `self`."),
            };

            vtable_entries += &format!("{name}: ({c_fn}, objective_rust::ffi::Selector),");
            vtable_setup += &format!(
                r#"
                let {name} = {{
                    let sel = objective_rust::ffi::get_selector("{selector}").unwrap();
                    let raw_func = objective_rust::ffi::get_method_impl({class}, sel).unwrap();
                    let func = unsafe {{ core::mem::transmute(raw_func) }};

                    (func, sel)
                }};
                "#
            );
            vtable_constructor += &format!("{name},");

            let fn_args = if *self_reference == SelfReference::None && args_with_types.len() > 2 {
                // skip over the `, `
                &args_with_types[2..]
            } else {
                args_with_types.as_str()
            };
            let instance_ptr = if *self_reference == SelfReference::None {
                "Self::get_objc_class()"
            } else {
                "self.0.as_ptr()"
            };
            struct_fns += &format!(
                "
                pub fn {name}({self_reference}{fn_args}){return_type_formatted} {{
                    {class_name}_VTABLE.with(|vtable| {{
                        let func = vtable.{name}.0;
                        let sel = vtable.{name}.1;

                        func({instance_ptr}, sel{args_no_types})
                    }})
                }}
                "
            );
        }

        write!(
            f,
            r#"
            struct {class_name}VTable {{
                class: objective_rust::ffi::Class,
                metaclass: objective_rust::ffi::Class,
                release: (
                    extern "C" fn(*mut {class_name}Instance, objective_rust::ffi::Selector),
                    objective_rust::ffi::Selector
                ),
                {vtable_entries}
            }}
            thread_local! {{
                static {class_name}_VTABLE: {class_name}VTable = {{
                    let class = objective_rust::ffi::get_class("{class_name}").unwrap();
                    let metaclass = objective_rust::ffi::get_metaclass("{class_name}").unwrap();
                    let release = {{
                        let sel = objective_rust::ffi::get_selector("release").unwrap();
                        let raw_func = objective_rust::ffi::get_method_impl(class, sel).unwrap();
                        let func = unsafe {{ core::mem::transmute(raw_func) }};

                        (func, sel)
                    }};

                    {vtable_setup}

                    {class_name}VTable {{
                        class,
                        metaclass,
                        release,
                        {vtable_constructor}
                    }}
                }};
            }}

            /// An opaqe type representing an Objective-C instance of [`{class_name}`].
            /// Class constructors should return a pointer to this type, and [`{class_name}`]
            /// stores a pointer to this type.
            pub struct {class_name}Instance(std::marker::PhantomData<()>);

            pub struct {class_name}(std::ptr::NonNull<{class_name}Instance>);

            impl {class_name} {{
                /// Attempts to create a new `{class_name}` from a pointer.
                ///
                /// # Safety
                /// - The pointer must point to a valid `{class_name}Instance`.
                /// - The pointer must be valid for at least as long as this instance lives.
                pub unsafe fn from_raw(ptr: core::ptr::NonNull<{class_name}Instance>) -> Self {{
                    Self(ptr)
                }}

                /// Get the underlying pointer to the actual Objective-C class instance.
                pub fn into_raw(&self) -> core::ptr::NonNull<{class_name}Instance> {{
                    self.0
                }}

                /// Returns the Objective-C class this struct binds to.
                pub fn get_objc_class() -> objective_rust::ffi::Class {{
                    {class_name}_VTABLE.with(|vtable| vtable.class.clone())
                }}

                /// Returns thie Objective-C metaclass for the class this struct binds to.
                pub fn get_objc_metaclass() -> objective_rust::ffi::Class {{
                    {class_name}_VTABLE.with(|vtable| vtable.metaclass.clone())
                }}

                {struct_fns}
            }}
            impl Drop for {class_name} {{
                fn drop(&mut self) {{
                    {class_name}_VTABLE.with(|vtable| vtable.release.0(self.0.as_ptr(), vtable.release.1) );
                }}
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
