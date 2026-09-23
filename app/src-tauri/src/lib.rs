// Ferry — Tauri app entry point
// Registers all Tauri commands and initialises plugins.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Portable modules — these build for Linux too, which is what lets the
// `ferry-restore` CLI exist (see src/restore_cli/main.rs). Everything below the
// cfg(windows) line touches diskpart, the registry, netsh or winget and is
// Windows-only by nature.
pub mod crypto;
pub mod hashing;
pub mod restore;
pub mod safety;
pub mod types;
pub mod ubuntu;
pub mod wifi;

#[cfg(windows)]
mod backup;
#[cfg(windows)]
mod browser;
#[cfg(windows)]
mod cloud;
#[cfg(windows)]
mod disk;
#[cfg(windows)]
mod download;
#[cfg(windows)]
mod inventory;
#[cfg(windows)]
mod proc;


#[cfg(all(windows, debug_assertions))]
use tauri::Manager;

#[cfg(windows)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // DevTools only when explicitly requested: an extra window next to
            // the app confuses non-technical users. Set FERRY_DEVTOOLS=1 locally.
            #[cfg(debug_assertions)]
            if std::env::var_os("FERRY_DEVTOOLS").is_some() {
                app.get_webview_window("main").unwrap().open_devtools();
            }
            #[cfg(not(debug_assertions))]
            let _ = app;
            Ok(())
        })
        // ── Disk ──────────────────────────────────────────────────────────
        .invoke_handler(tauri::generate_handler![
            disk::enumerate::list_removable_drives,
            disk::partition::prepare_usb,
            disk::bootloader::write_bootloader,
            // ── Backup ────────────────────────────────────────────────────
            backup::scan::scan_user_files,
            backup::scan::list_usb_backup_files,
            backup::paths::migration_profile,
            backup::copy::copy_files_to_usb,
            backup::checksum::verify_backup,
            backup::checksum::read_manifest,
            // ── Encrypt ───────────────────────────────────────────────────
            crypto::encrypt::encrypt_backup,
            crypto::decrypt::decrypt_backup,
            // ── Download ──────────────────────────────────────────────────
            download::fetch::download_os_image,
            download::custom::inspect_custom_iso,
            download::custom::copy_custom_iso,
            download::sources::list_os_sources,
            // ── Inventory ─────────────────────────────────────────────────
            inventory::apps::scan_installed_apps,
            inventory::network::scan_network_adapters,
            inventory::store::scan_store_apps,
            inventory::picker::resolve_app_tiers,
            inventory::picker::install_app,
            inventory::save::save_inventory,
            inventory::save::read_inventory,
            // ── Browser ───────────────────────────────────────────────────
            browser::chrome::backup_chromium_data,
            browser::firefox::backup_firefox_data,
            // ── Wi-Fi ─────────────────────────────────────────────────────
            wifi::export::export_wifi_profiles,
            wifi::import::import_wifi_profiles,
            wifi::list::list_wifi_profiles,
            wifi::list::wifi_password,
            // ── Ubuntu restore tool on the USB ────────────────────────────
            backup::restore_tool::copy_restore_tool,
            // ── Restore ───────────────────────────────────────────────────
            restore::detect::find_backup_on_usb,
            restore::copy::restore_files,
            // -- Cloud Backup (managed, free) --
            cloud::b2::upload_backup_b2,
            cloud::b2::download_backup_b2,
            cloud::b2::delete_cloud_backup,
            cloud::b2::cloud_status,
            cloud::b2::cloud_sign_in_start,
            cloud::b2::cloud_sign_in_verify,
            cloud::b2::cloud_list_backups,
            cloud::b2::cloud_start_anonymous,
            cloud::b2::cloud_sign_in_code,
            cloud::b2::save_restore_code,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Ferry");
}

