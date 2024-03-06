# objective-rust

objective-rust is a blazingly-fast, dependency-free Objective-C~~ringe~~ FFI library for Rust. Unlike other Objective-C crates, it doesn't add messaging macros or awkward class types; instead, objective-rust uses macros to expose Objective-C classes as Rust structs. Class instances can be stored as regular variables, and methods work exactly the same as they do in Rust. Here's an example:

```rust
// The `objrs` attribute macro generates FFI
use objective_rust::objrs;

// Declare Objective-C types with the `#[objrs]` macro and
// an `extern "objc"` block
#[objrs]
extern "objc" {
    // The class this block describes
    type NSApplication;

    // Methods for instances of the class
    fn run(&self);
    // Static methods for the class itself
    fn sharedApplication() -> *mut Self;

    // You can also change which selector objective-rust will
    // call internally. In Rust, the function's name will still
    // match the name in the function declaration.
    #[selector = "sharedApplication"]
    fn shared() -> *mut Self;
}

fn main() {
    let shared = NSApplication::shared();
    // `from_raw` is automatically added to Objective-C types, and
    // is the main way to create those types.
    let shared = unsafe { NSApplication::from_raw(shared).unwrap() };
    shared.run();
}
```

This API allows Objective-C types to feel almost native in Rust, and never relies on message passing (even internally, objective-rust stores a pointer to the actual C function for a method, and calls that instead of sending an Objective-C message).

Objective-C instances also act like normal Rust instances; when it is dropped, the `dealloc` method is called, preventing memory leaks.

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

    // This isn't in an `extern "objc"` block, so it doesn't get
    // processed by objective-rust.
    struct SomeType {}
}
```

In the future, if crate-level macros are ever stabilised, you can add `#![objective_rust::objrs]` to the top of a crate, and then use `extern "objc"` anywhere in that crate to generate FFI.

There is a [demo that opens a window with AppKit](examples/appkit.rs) in the examples folder.

# Limitations

- objective-rust doesn't support borrows; pointers should be used instead. I'm not yet sure how borrows across FFI could affect safety guarantees, so only pointers are supported, and safety guarantees are not made.
- objective-rust can currently only import existing Objective-C classes. In the future, I'd like to support exporting Rust structs as Objective-C classes, but that's not been added yet.
- Protocols can't be imported yet, but in the future I'd like to support importing them as traits.
