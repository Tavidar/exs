// Exsul (new core) — Tauri entry point.
//
// Структура проводки перенесена из Exsul (panic hook → preflight → recovery DB →
// HLC → команды → backup-on-close), но очищена от старого домена (drive/ws/orders/
// flowers/...). Чистое ядро под AI-ассистента.

mod ai;
mod business;
mod commands;
mod db;
mod events;
mod files;
mod preflight;
mod search;
mod sync;

use sync::hlc::HybridLogicalClock;
use tauri::Manager;

/// Panic log path: %LOCALAPPDATA%\com.exsul.app\startup-panic.log. Resolved via
/// env so it works even before AppHandle exists.
fn panic_log_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA").or_else(|| std::env::var_os("APPDATA"))?;
    let mut p = std::path::PathBuf::from(base);
    p.push("com.exsul.app");
    let _ = std::fs::create_dir_all(&p);
    p.push("startup-panic.log");
    Some(p)
}

/// Global panic hook installed BEFORE Tauri runs — catches setup() panics that
/// would otherwise exit the process silently (flash-and-die).
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "<non-string panic payload>".to_string());
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown>".to_string());
        let body = format!(
            "[{}] PANIC: {}\nLocation: {}\nBacktrace:\n{}\n\n",
            chrono::Utc::now().to_rfc3339(),
            message,
            location,
            std::backtrace::Backtrace::force_capture(),
        );
        if let Some(path) = panic_log_path() {
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
            {
                use std::io::Write;
                let _ = f.write_all(body.as_bytes());
            }
        }
        eprintln!("{}", body);
    }));
}

#[cfg(windows)]
fn show_fatal_dialog(title: &str, body: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
    let title_w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let body_w: Vec<u16> = body.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            body_w.as_ptr(),
            title_w.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
#[cfg(not(windows))]
fn show_fatal_dialog(title: &str, body: &str) {
    eprintln!("[fatal] {}: {}", title, body);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_panic_hook();

    if let Err(e) = preflight::run_preflight() {
        eprintln!("Preflight failed: {:?}", e);
        std::process::exit(1);
    }

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
                let _ = window.unminimize();
            }
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("exsul".into()),
                    }),
                ])
                .max_file_size(5_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            log::info!("Exsul starting up (version {})", app.package_info().version);

            // Explicit native-dialog selections for bounded local AI context.
            // Kept in memory: no path permissions survive an app restart.
            app.manage(files::local_context::ContextFileAccess::default());

            // NEVER returns Err; worst case in-memory DB. Recovery state is
            // persisted into local_config for the frontend banner.
            let recovery_state = db::init_with_recovery(app.handle());
            if !matches!(recovery_state, db::RecoveryState::Healthy) {
                log::warn!("DB initialized with recovery state: {:?}", recovery_state);
            }

            // HLC seeded from the DB node_id.
            let node_id = {
                let db_state = app.handle().state::<db::Database>();
                let conn = db_state.conn.lock().unwrap_or_else(|p| p.into_inner());
                match db::queries::get_node_id(&conn) {
                    Ok(id) => id,
                    Err(_) => {
                        let fresh = uuid::Uuid::new_v4().to_string();
                        let _ = conn.execute(
                            "INSERT OR REPLACE INTO local_config (key, value) VALUES ('node_id', ?1)",
                            [&fresh],
                        );
                        fresh
                    }
                }
            };
            app.handle().manage(HybridLogicalClock::new(node_id));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ── Inventory ──
            commands::inventory::add_item,
            commands::inventory::update_item,
            commands::inventory::get_items,
            commands::inventory::get_item,
            commands::inventory::record_sale,
            commands::inventory::adjust_stock,
            commands::inventory::change_price,
            commands::inventory::save_item_image,
            commands::inventory::delete_item,
            commands::inventory::delete_all_items,
            commands::inventory::duplicate_item,
            commands::inventory::get_inventory_summary,
            commands::inventory::seed_demo_items,
            // ── Search ──
            commands::search::search_products,
            commands::search::rebuild_search_index,
            // ── Explicit local AI context ──
            commands::context_files::select_context_files,
            commands::context_files::read_context_file,
            commands::context_files::forget_context_file,
            commands::context_files::clear_context_files,
            // ── AI Gateway ──
            commands::ai::assistant_query,
            commands::ai::analyze_item_image,
            commands::ai::ai_get_status,
            commands::ai::ai_set_provider,
            commands::ai::ai_set_provider_key,
            commands::ai::ai_delete_provider_key,
            // ── Business / ELS (Entrepreneur Launch System) ──
            commands::business::business_discovery_questions,
            commands::business::business_start_session,
            commands::business::business_get_session,
            commands::business::business_list_sessions,
            commands::business::business_submit_answer,
            commands::business::business_generate_profile,
            // ── Categories ──
            commands::categories::get_categories,
            commands::categories::create_category,
            commands::categories::delete_category,
            // ── Audit / events ──
            commands::audit::get_audit_logs,
            commands::audit::get_events,
            // ── Backup ──
            commands::backup::export_backup,
            commands::backup::import_backup,
            // ── System ──
            commands::system::get_app_version,
            commands::system::get_recovery_state,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let handle = window.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    let h = handle.clone();
                    let res = tokio::task::spawn_blocking(move || files::backup::create_backup(&h)).await;
                    match res {
                        Ok(Ok(path)) => log::info!("Auto-backup created on close: {:?}", path),
                        Ok(Err(e)) => log::error!("Auto-backup failed: {}", e),
                        Err(e) => log::error!("Auto-backup task panicked: {}", e),
                    }
                });
            }
        })
        .build(tauri::generate_context!());

    match result {
        Ok(app) => app.run(|_, _| {}),
        Err(e) => {
            log::error!("Tauri build failed: {}", e);
            show_fatal_dialog(
                "Exsul: კრიტიკული შეცდომა",
                &format!(
                    "გრაფიკული გარემოს ინიციალიზაცია ვერ მოხერხდა.\n\nDetails: {}\n\n\
                     Failed to initialize the GUI runtime.",
                    e
                ),
            );
            std::process::exit(2);
        }
    }
}
