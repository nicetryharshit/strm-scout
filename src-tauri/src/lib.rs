mod strm;

use std::path::PathBuf;
use std::sync::Mutex;
use strm::{LoadedStream, StreamInfo};
use tauri::ipc::Response;

struct AppState(Mutex<Option<LoadedStream>>);

#[tauri::command]
fn load_strm(path: String, state: tauri::State<AppState>) -> Result<StreamInfo, String> {
    let stream = LoadedStream::open(PathBuf::from(path))?;
    let info = stream.info();
    *state.0.lock().map_err(|_| "Stream state is unavailable".to_string())? = Some(stream);
    Ok(info)
}

#[tauri::command]
fn decode_frame(index: usize, state: tauri::State<AppState>) -> Result<Response, String> {
    let guard = state.0.lock().map_err(|_| "Stream state is unavailable".to_string())?;
    let stream = guard.as_ref().ok_or_else(|| "No STRM file is open".to_string())?;
    Ok(Response::new(stream.decode_frame_png(index)?))
}

#[tauri::command]
fn export_frame(directory: String, index: usize, state: tauri::State<AppState>) -> Result<String, String> {
    let guard = state.0.lock().map_err(|_| "Stream state is unavailable".to_string())?;
    let stream = guard.as_ref().ok_or_else(|| "No STRM file is open".to_string())?;
    let path = stream.export_frame(PathBuf::from(directory), index)?;
    Ok(format!("Exported {}", path.display()))
}

#[tauri::command]
fn export_ranges(directory: String, range_indexes: Vec<usize>, state: tauri::State<AppState>) -> Result<String, String> {
    let guard = state.0.lock().map_err(|_| "Stream state is unavailable".to_string())?;
    let stream = guard.as_ref().ok_or_else(|| "No STRM file is open".to_string())?;
    let count = stream.export_ranges(PathBuf::from(directory), &range_indexes)?;
    Ok(format!("Exported {count} PNG frames"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![load_strm, decode_frame, export_frame, export_ranges])
        .run(tauri::generate_context!())
        .expect("failed to run STRM Inspector");
}
