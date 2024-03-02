pub use objective_rust_macros::*;

#[cfg(not(target_os = "macos"))]
compile_error!("objective-rust only supports macOS");

/// Objective-C's boolean type.
#[repr(transparent)]
pub struct Bool(std::ffi::c_char);
impl Bool {
    pub const TRUE: Self = Self(1);
    pub const FALSE: Self = Self(0);

    pub const YES: Self = Self::TRUE;
    pub const NO: Self = Self::FALSE;
}

pub mod ffi {
    use std::{ffi::CString, ptr::NonNull};
    type Ptr = NonNull<()>;

    /// An Objective-C class.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    pub struct Class(Ptr);
    /// An instance of an Objective-C class.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    pub struct Instance(Ptr);
    /// A pointer to the implementation of an Objective-C function.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    pub struct Implementation(Ptr);
    /// A selector for an Objective-C function.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    pub struct Selector(Ptr);
    /// A structure that defines an Objective-C method.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    pub struct Method(Ptr);

    /// Returns a [`Class`] if one exists for `name`. Otherwise returns `None`.
    ///
    /// https://developer.apple.com/documentation/objectivec/1418952-objc_getclass?language=objc
    pub fn get_class(name: &str) -> Option<Class> {
        let name = CString::new(name).ok()?;
        let ptr = unsafe { objc_getClass(name.as_ptr()) };

        Some(Class(Ptr::new(ptr)?))
    }

    pub fn get_metaclass(name: &str) -> Option<Class> {
        let name = CString::new(name).ok()?;
        let ptr = unsafe { objc_getMetaClass(name.as_ptr()) };

        Some(Class(Ptr::new(ptr)?))
    }

    pub fn get_selector(name: &str) -> Option<Selector> {
        let name = CString::new(name).ok()?;
        let ptr = unsafe { sel_getUid(name.as_ptr()) };

        Some(Selector(Ptr::new(ptr)?))
    }

    #[inline(always)]
    pub fn get_method_impl(class: Class, method: Selector) -> Option<Implementation> {
        let ptr = unsafe { class_getMethodImplementation(class, method) };
        Some(Implementation(Ptr::new(ptr)?))
    }

    #[link(name = "objc")]
    extern "C" {
        fn class_getMethodImplementation(cls: Class, name: Selector) -> *mut ();
        fn objc_getClass(name: *const i8) -> *mut ();
        fn objc_getMetaClass(name: *const i8) -> *mut ();
        fn sel_getUid(name: *const i8) -> *mut ();
    }
}
