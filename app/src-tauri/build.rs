fn main() {
    // Ferry needs administrator rights to run at all (diskpart, raw partition
    // writes, bootloader install) — see windows-app-manifest.xml for why this
    // is embedded rather than left to the user to discover.
    #[cfg(windows)]
    {
        let windows = tauri_build::WindowsAttributes::new()
            .app_manifest(include_str!("windows-app-manifest.xml"));
        let attrs = tauri_build::Attributes::new().windows_attributes(windows);
        tauri_build::try_build(attrs).expect("failed to run tauri-build");
    }
    #[cfg(not(windows))]
    tauri_build::build();
}

