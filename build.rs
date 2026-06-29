//! Embeds the application icon (and version metadata) into the Windows .exe so
//! it shows up in Explorer, shortcuts, and the pinned taskbar. The live window
//! / taskbar icon is set separately at runtime via iced's window settings.

fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        if let Err(e) = res.compile() {
            // Don't fail the build if the resource compiler is unavailable;
            // the runtime window icon still works.
            println!("cargo:warning=icon resource not embedded: {e}");
        }
    }
    println!("cargo:rerun-if-changed=assets/icon.ico");
}
