use serde::{Deserialize, Serialize};
use std::io::{self, Read};
use std::sync::Mutex;
use tauri::{Manager, State};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClipboardItem {
    pub label: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppData {
    pub items: Vec<ClipboardItem>,
    pub mode: String,
    pub is_default_data: bool,
}

pub struct AppState {
    pub data: Mutex<Option<AppData>>,
}

#[tauri::command]
fn get_clipboard_data(state: State<AppState>) -> Result<AppData, String> {
    let data = state.data.lock().unwrap();
    
    match &*data {
        Some(app_data) => Ok(app_data.clone()),
        None => Err("No data available".to_string()),
    }
}

fn read_stdin_data() -> Result<AppData, String> {
    // 標準入力からデータを読み取る
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .map_err(|e| format!("Failed to read from stdin: {}", e))?;

    // 読み取ったデータをログ出力
    println!("Input data received: {}", buffer);

    // 空の入力をチェックして、デフォルトデータを使用
    let is_empty_input = buffer.trim().is_empty();
    if is_empty_input {
        println!("Empty input detected, using default data");
        buffer = r#"[
    {"label": "ラベル1", "text": "テキスト1"},
    {"label": "ラベル2", "text": "テキスト2"}
]"#.to_string();
        println!("Input data received: {}", buffer);
    }

    // CLI 引数を取得
    let args: Vec<String> = std::env::args().collect();
    let is_default_data = buffer.trim().starts_with('[');
    
    let mode = if args.contains(&"--multiline".to_string()) || args.contains(&"-m".to_string()) {
        "multiline"
    } else if args.contains(&"--split-empty-line".to_string()) || args.contains(&"-s".to_string()) {
        "split-empty-line"
    } else if args.contains(&"--json".to_string()) || args.contains(&"-j".to_string()) || is_default_data {
        "json"
    } else {
        "normal"
    };

    let items = match mode {
        "multiline" => {
            buffer
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| ClipboardItem {
                    label: line.to_string(),
                    text: line.to_string(),
                })
                .collect()
        }
        "split-empty-line" => {
            buffer
                .split("\n\n")
                .filter(|section| !section.trim().is_empty())
                .map(|section| {
                    let lines: Vec<&str> = section.lines().collect();
                    let label = lines.first().unwrap_or(&"").to_string();
                    ClipboardItem {
                        label,
                        text: section.to_string(),
                    }
                })
                .collect()
        }
        "json" => {
            serde_json::from_str::<Vec<ClipboardItem>>(&buffer)
                .map_err(|e| format!("Failed to parse JSON: {}", e))?
        }
        _ => {
            // normal mode
            vec![ClipboardItem {
                label: buffer.trim().to_string(),
                text: buffer.trim().to_string(),
            }]
        }
    };

    println!("Processing mode: {}", mode);
    println!("Created {} clipboard items", items.len());

    Ok(AppData {
        items,
        mode: mode.to_string(),
        is_default_data: is_empty_input,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 起動時に標準入力を読み取る
    let initial_data = read_stdin_data().ok();
    
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_cli::init())?;
            
            // アプリケーションステートを設定
            app.manage(AppState {
                data: Mutex::new(initial_data),
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_clipboard_data])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}