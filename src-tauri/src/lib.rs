mod commands;
mod db;
pub mod mqtt_manager;
mod settings;
pub mod updater;

use db::Db;
use mqtt_manager::MqttManager;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::open(&dir.join("mqttxx.db"))?;
            app.manage(Db(std::sync::Mutex::new(conn)));
            app.manage(MqttManager::new(app.handle().clone()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::connection_list,
            commands::connection_save,
            commands::connection_delete,
            commands::subscription_list,
            commands::subscription_add,
            commands::subscription_delete,
            commands::connect,
            commands::disconnect,
            commands::subscribe_live,
            commands::unsubscribe_live,
            commands::publish,
            commands::publish_history_list,
            commands::publish_history_set_pinned,
            commands::publish_history_clear,
            updater::check_update,
            updater::download_update,
            updater::install_update,
            commands::message_list,
            commands::message_clear,
            commands::pick_cert_file,
            commands::settings_get,
            commands::settings_set,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
