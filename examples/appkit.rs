//! Makes a window on macOS with AppKit. This doesn't process any events.

use objective_rust::{objrs, Bool as ObjcBool};

fn main() {
    println!("Getting shared NSApp");
    let shared = NSApplication::shared();
    let ns_app = unsafe { NSApplication::from_raw(shared).unwrap() };

    println!("Creating window");
    let window = NSWindow::alloc();
    let mut window = unsafe { NSWindow::from_raw(window).unwrap() };
    let mut style_mask = NSWindowStyleMask::default();
    style_mask.closable().resizable().titled();
    window.init(
        NSRect {
            origin: NSPoint { x: 0.0, y: 0.0 },
            size: NSSize {
                width: 600.0,
                height: 400.0,
            },
        },
        style_mask,
        2,
        ObjcBool::FALSE,
    );
    window.make_key(std::ptr::null_mut());

    println!("Calling run");
    ns_app.run();

    unreachable!()
}

#[objrs]
mod ffi {
    use super::*;

    extern "objc" {
        type NSApplication;

        #[selector = "sharedApplication"]
        fn shared() -> *mut Self;
        fn run(&self);
    }
    extern "objc" {
        type NSWindow;

        fn alloc() -> *mut Self;

        #[selector = "initWithContentRect:styleMask:backing:defer:"]
        fn init(
            &mut self,
            content_rect: NSRect,
            style_mask: NSWindowStyleMask,
            backing_store: u64,
            defer: ObjcBool,
        );

        #[selector = "makeKeyAndOrderFront:"]
        fn make_key(&mut self, sender: *mut ());
    }

    #[repr(C)]
    pub struct NSPoint {
        pub x: f64,
        pub y: f64,
    }
    #[repr(C)]
    pub struct NSSize {
        pub width: f64,
        pub height: f64,
    }
    #[repr(C)]
    pub struct NSRect {
        pub origin: NSPoint,
        pub size: NSSize,
    }

    #[derive(Default)]
    #[repr(transparent)]
    pub struct NSWindowStyleMask(u64);

    #[allow(dead_code)] // Every non-deprecated style mask is listed here, for completeness' sake.
    impl NSWindowStyleMask {
        pub fn borderless(&mut self) -> &mut Self {
            self.0 = 0;
            self
        }
        pub fn titled(&mut self) -> &mut Self {
            self.0 |= 1 << 0;
            self
        }
        pub fn closable(&mut self) -> &mut Self {
            self.0 |= 1 << 1;
            self
        }
        pub fn miniaturizable(&mut self) -> &mut Self {
            self.0 |= 1 << 2;
            self
        }
        pub fn resizable(&mut self) -> &mut Self {
            self.0 |= 1 << 3;
            self
        }
        pub fn unified_title_and_toolbar(&mut self) -> &mut Self {
            self.0 |= 1 << 12;
            self
        }
        pub fn fullscreen(&mut self) -> &mut Self {
            self.0 |= 1 << 14;
            self
        }
        pub fn full_size_content_view(&mut self) -> &mut Self {
            self.0 |= 1 << 15;
            self
        }
        pub fn utility(&mut self) -> &mut Self {
            self.0 |= 1 << 4;
            self
        }
        pub fn doc_modal(&mut self) -> &mut Self {
            self.0 |= 1 << 6;
            self
        }
        pub fn non_activating_panel(&mut self) -> &mut Self {
            self.0 |= 1 << 7;
            self
        }
        pub fn hud_window(&mut self) -> &mut Self {
            self.0 |= 1 << 13;
            self
        }
    }

    // Without this, Rust won't link to AppKit and AppKit classes won't get loaded.
    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}
}
use ffi::*;
