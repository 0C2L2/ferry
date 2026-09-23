fn main() {
    // Two different "is this Windows?" questions have to be answered here, and
    // conflating them breaks cross-compilation:
    //
    //   * `cfg(windows)` in a build script describes the HOST, because build
    //     scripts are compiled and run on the machine doing the building. It
    //     gates whether `tauri_build` even exists to call.
    //   * `CARGO_CFG_TARGET_OS` describes the TARGET being built for. Building
    //     the Linux `ferry-restore` from a Windows machine has host=windows and
    //     target=linux, and tauri-build panics with "missing cargo:dev
    //     instruction" if it runs in that case.
    #[cfg(windows)]
    {
        let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
        if target_os == "windows" {
            // Ferry needs administrator rights to run at all (diskpart, raw
            // partition writes, bootloader install) — see
            // windows-app-manifest.xml for why this is embedded rather than
            // left to the user to discover.
            let windows = tauri_build::WindowsAttributes::new()
                .app_manifest(include_str!("windows-app-manifest.xml"));
            let attrs = tauri_build::Attributes::new().windows_attributes(windows);
            tauri_build::try_build(attrs).expect("failed to run tauri-build");
        }
    }
}

