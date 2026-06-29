//! Embeds the application icon (and version metadata) into the Windows .exe so
//! it shows up in Explorer, shortcuts, and the pinned taskbar. The live window
//! / taskbar icon is set separately at runtime via iced's window settings.

fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        // Require admin elevation only for the shipped release build.
        if std::env::var("PROFILE").unwrap_or_default() == "release" {
            res.set_manifest_file("assets/app.manifest");
        }
        if let Err(e) = res.compile() {
            println!("cargo:warning=icon/manifest resource not embedded: {e}");
        }
    }
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=assets/app.manifest");
}
