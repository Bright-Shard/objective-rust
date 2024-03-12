# objective-rust

objective-rust is a blazingly-fast, dependency-free Objective-C~~ringe~~ FFI library for Rust. Unlike other Objective-C libraries, objective-rust allows you to use Objective-C classes as regular Rust types; it doesn't try to introduce weird Objective-C syntax into Rust.

Here's a simple demo program, which imports and uses the `NSApplication` class from AppKit:

```rust
// The `objrs` attribute macro, which generates FFI for you
use objective_rust::objrs;
use std::ptr::NonNull;

// Declare Objective-C types with the `#[objrs]` macro and an
// `extern "objc"` block
#[objrs]
extern "objc" {
    // The class to import
    type NSApplication;

    // Methods for instances of the class (takes &self or &mut self)
    fn run(&self);
    // Static methods for the class itself (doesn't take self)
    fn sharedApplication() -> *mut Self;

    // You can also change which method objective-rust will
    // call internally, with the selector attribute
    #[selector = "sharedApplication"]
    fn shared() -> *mut Self;
}

fn main() {
    // Call class methods just like associated functions in Rust
    let shared = NSApplication::shared();
    // `from_raw` is added by objective-rust, and just converts a pointer
    // to an instance into a useable Rust type
    // It requires a non-null pointer
    let shared = NonNull::new(shared).unwrap();
    let shared = unsafe { NSApplication::from_raw(shared) };
    // Call instance methods just like methods in Rust
    shared.run();
}

// Without this, Rust won't link to AppKit and AppKit classes won't get loaded.
// This doesn't import anything from AppKit. It just links to it so we can use
// its classes.
#[link(name = "AppKit", kind = "framework")]
extern "C" {}
```

Everything from the way types and methods are declared (in `extern` blocks) to the way they're used (associated functions and methods) to their behaviour (`release` is automatically called when an instance is dropped) is designed to feel like native Rust. The only real difference is having to construct an instance from a raw pointer.

By the way, the `objrs` macro also works on entire modules:

```rust
#[objrs]
mod ffi {
    // All `extern "objc"` blocks in the module will get parsed.
    extern "objc" {
        type NSApplication;

        #[selector = "sharedApplication"]
        fn shared() -> *mut Self;
        fn run(&self);
    }

    // This isn't in an `extern "objc"` block, so it is ignored/not processed
    pub struct SomeType {}
}

use ffi::{NSApplication, SomeType};
```

In the future, if crate-level macros are ever stabilised, you can add `#![objective_rust::objrs]` to the top of a crate, and then use `extern "objc"` anywhere in that crate to generate FFI.

# Examples

- The [AppKit example](examples/appkit.rs) - this opens a window on macOS using AppKit. It doesn't handle events or render anything, but does show objective-rust working.
- [Loki](https://github.com/loki-chat)'s [loki-mac](https://github.com/loki-chat/lokinit/tree/main/loki-mac) and [lokinit](https://github.com/loki-chat/lokinit) libraries - if you've not heard of Lokinit, it's a work-in-progress windowing library with almost no dependencies. Unlike other windowing libraries, Lokinit puts your app in control of the event loop, instead of letting the OS control the thread. objective-rust was made for Lokinit's macOS backend.

# Limitations

- objective-rust doesn't support borrows; pointers should be used instead. I'm not yet sure how borrows across FFI could affect safety guarantees, so only pointers are supported, and safety guarantees are not made.
- objective-rust can currently only import existing Objective-C classes. In the future, I'd like to support exporting Rust structs as Objective-C classes, but that's not been added yet.
- Protocols can't be imported yet, but in the future I'd like to support importing them as traits.

# Internal Details / How it Works

Note: If you want to see this in action, run `cargo install cargo-expand`, then `cargo expand` on any objective-rust project. It'll show the macro output.

## Overview

objective-rust uses Apple's [Objective-C Runtime API](https://developer.apple.com/documentation/objectivec?language=objc) to interact with Objective-C classes. Unlike other Objective-C crates, or even Objective-C itself, it doesn't rely on message passing; instead, objective-rust uses the API to get the underlying C function for Objective-C methods and calls that function directly.

objective-rust will use thread local storage to store pointers to any Objective-C methods imported via the `objrs` macro. When you call a method, it loads that function pointer from thread local storage, and calls the function with the appropriate arguments.

## Nitty Gritty

When you declare a type in an `extern "objc"` block, objective-rust will generate these three structs for it (with <class> representing the class name):

- `<class>`: A struct with the same name as the class. This has all of the methods implemented for it, and is the type you use in your program. It's the "Rust wrapper type" for an Objective-C class.
- `<class>Instance`: An opaque type that represents an Objective-C instance of the class you're importing. This just exists to semantically separate the Objective-C type from the Rust wrapper type; it has no methods or other functionality.
- `<class>VTable`: A struct used by objective-rust to store function pointers for all of `<class>`'s methods.

When you declare a function in an `extern "objc"` block, objective-rust adds a field to the `<class>VTable` struct for that function. The field stores the selector for that function and a pointer to the function itself. objective-rust will then store an instance of `<class>VTable` in thread-local storage.

When you call a method in `<class>`, objective-rust gets the function pointer and selector for the function from the `<class>VTable` instance in thread-local storage, and calls the function with all the arguments you give it.

## Other Notes

Stuff that may be helpful to anyone else working with the Objective-C runtime:

- All Objective-C methods are implemented as C functions under the hood. All of those functions have this signature: `extern "C" fn(instance: *mut Self, selector: Selector, <function arguments>)` - in short, the first argument is always the instance this method is running on (the `self` pointer), the second argument is the selector of the function, and anything after that is the function's actual arguments (if it has any).
- You can get the underlying C function for an Objective-C method with the [`class_getMethodImplementation`](https://developer.apple.com/documentation/objectivec/1418811-class_getmethodimplementation?language=objc) function.
- The C function signature described above also applies to class/static methods. For these methods, the instance is the _class itself_, instead of a class instance. In addition, the function is implemented for the class' metaclass, not the class. So, to load the function with `class_getMethodImplementation`, you pass the metaclass for the `class` argument. You can get a metaclass with [`objc_getMetaClass`](https://developer.apple.com/documentation/objectivec/1418721-objc_getmetaclass?language=objc).
- Objective-C properties are actually just implemented as a getter function and a setter function. So you can use this same function loading technique to read properties.
