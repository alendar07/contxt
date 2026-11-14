mod ai;

use std::sync::Mutex;

use screenshots::image::ImageFormat;
use screenshots::Screen;

use tauri::{Emitter, Manager};

use serde::Serialize;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow,
    GetWindowTextW,
    GetWindowThreadProcessId,
};

use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_global_shortcut::{
    Code,
    Modifiers,
    ShortcutState,
};

struct AppState {
    screenshot: Mutex<Option<Vec<u8>>>,
    active_window: Mutex<Option<ActiveWindowContext>>,
}

#[derive(Clone, Serialize)]
struct ActiveWindowContext {
    title: String,
    process_id: u32,
}

#[tauri::command]
fn read_active_window_context() -> Result<ActiveWindowContext, String> {
    unsafe {
        let hwnd = GetForegroundWindow();

        if hwnd.0.is_null() {
            return Err("No foreground window detected".to_string());
        }

        let mut title_buffer = [0u16; 1024];
        let length = GetWindowTextW(hwnd, &mut title_buffer);

        let title = String::from_utf16_lossy(
            &title_buffer[..length.max(0) as usize],
        );

        let mut process_id = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));

        Ok(ActiveWindowContext {
            title,
            process_id,
        })
    }
}

#[tauri::command]
fn get_active_window_context(
    state: tauri::State<AppState>,
) -> Result<ActiveWindowContext, String> {
    let stored = state
        .active_window
        .lock()
        .map_err(|_| "Failed to access active window state".to_string())?;

    stored
        .clone()
        .ok_or_else(|| "No active window context captured yet".to_string())
}

fn capture_current_screen() -> Result<Vec<u8>, String> {

    let screens = Screen::all()
        .map_err(|error| error.to_string())?;

    let screen = screens
        .first()
        .ok_or_else(|| "No screen detected".to_string())?;

    let image = screen
        .capture()
        .map_err(|error| error.to_string())?;

    let mut bytes = Vec::new();

    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            ImageFormat::Png,
        )
        .map_err(|error| error.to_string())?;


    Ok(bytes)
}

#[tauri::command]
fn get_screenshot(
    state: tauri::State<AppState>,
) -> Result<Option<Vec<u8>>, String> {
    let screenshot = state
        .screenshot
        .lock()
        .map_err(|_| {
            "Failed to access screenshot state".to_string()
        })?;

    Ok(screenshot.clone())
}

#[tauri::command]
fn has_screenshot(
    state: tauri::State<AppState>,
) -> Result<bool, String> {
    let screenshot = state
        .screenshot
        .lock()
        .map_err(|_| {
            "Failed to access screenshot state".to_string()
        })?;

    Ok(screenshot.is_some())
}

#[tauri::command]
async fn ask_ai(
    question: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let screenshot = {
        let stored = state
            .screenshot
            .lock()
            .map_err(|_| {
                "Failed to access screenshot state".to_string()
            })?;

        stored.clone()
    };

    ai::ask_with_screenshot(question, screenshot).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if !shortcut.matches(
                        Modifiers::CONTROL | Modifiers::SHIFT,
                        Code::Space,
                    ) || event.state != ShortcutState::Pressed
                    {
                        return;
                    }

                    let Some(window) =
                        app.get_webview_window("main")
                    else {
                        return;
                    };

                    let visible =
                        window.is_visible().unwrap_or(false);

                    if visible {
                        let _ = window.hide();
                        return;
                    }

                    // Capture the app that was active before Context takes focus.
                    match read_active_window_context() {
                        Ok(context) => {
                            if let Some(state) = app.try_state::<AppState>() {
                                if let Ok(mut stored) = state.active_window.lock() {
                                    *stored = Some(context);
                                }
                            }

                            let _ = app.emit("context-active-window-ready", ());
                        }
                        Err(error) => {
                            eprintln!("ACTIVE WINDOW DETECTION FAILED: {}", error);
                        }
                    }

                    // Now show Context.
                    let _ = window.show();
                    let _ = window.set_focus();

                    // Capture asynchronously.
                    let app_handle = app.clone();

                    std::thread::spawn(move || {                    

                        match capture_current_screen() {
                            Ok(screenshot) => {

                                if let Some(state) =
                                    app_handle.try_state::<AppState>()
                                {
                                    if let Ok(mut stored) =
                                        state.screenshot.lock()
                                    {
                                        *stored =
                                            Some(screenshot);
                                    }
                                }


                                let _ = app_handle.emit(
                                    "context-screenshot-ready",
                                    (),
                                );
                            }

                            Err(error) => {
                                eprintln!(
                                    "SCREEN CAPTURE FAILED: {}",
                                    error
                                );
                            }
                        }
                    });
                })
                .build(),
        )
        .manage(AppState {
            screenshot: Mutex::new(None),
            active_window: Mutex::new(None),
        })
                
        .invoke_handler(
            tauri::generate_handler![
                has_screenshot,
                get_screenshot,
                ask_ai,
                get_active_window_context
            ]
        )
        .setup(|app| {
            if let Some(window) =
                app.get_webview_window("main")
            {
                let _ = window.hide();
            }

            app.global_shortcut()
                .register("CTRL+SHIFT+SPACE")?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}