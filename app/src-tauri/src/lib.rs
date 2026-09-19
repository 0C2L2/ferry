// Ferry — Tauri app entry point
// Registers all Tauri commands and initialises plugins.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod types;
mod safety;
mod disk;
mod backup;
mod crypto;
mod download;
mod inventory;
mod browser;
mod wifi;
mod restore;
mod cloud;

use tauri::Manager;

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
            download::sources::list_os_sources,
            // ── Inventory ─────────────────────────────────────────────────
            inventory::apps::scan_installed_apps,
            inventory::drivers::scan_drivers,
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
            // ── Restore ───────────────────────────────────────────────────
            restore::detect::find_backup_on_usb,
            restore::copy::restore_files,
            // ── Cloud Backup ──────────────────────────────────────────────
            cloud::b2::upload_backup_b2,
            cloud::b2::download_backup_b2,
            cloud::b2::delete_cloud_backup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Ferry");
}

