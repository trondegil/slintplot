//! The app icon in the macOS Dock. Slint's `Window.icon` sets the taskbar
//! icon on Windows and Linux, but macOS takes the Dock icon from the
//! application, so it is set there.

#[cfg(target_os = "macos")]
pub fn set() {
    use objc2::{AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let data = NSData::with_bytes(include_bytes!("../icon.png"));
    if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
        // SAFETY: called on the main thread (checked above) with a valid image.
        unsafe { NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&image)) };
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set() {}
