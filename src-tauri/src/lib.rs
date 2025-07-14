use serde::{Deserialize, Serialize};
use std::io::{self, Read};
use std::sync::Mutex;
use tauri::{Manager, State};
use clap::Parser;

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

#[derive(Parser)]
#[command(name = "clipboard-palette")]
#[command(about = "A clipboard palette application")]
struct Args {
    #[arg(short = 'm', long = "multiline", help = "Multiline mode")]
    multiline: bool,

    #[arg(short = 'j', long = "json", help = "JSON mode")]
    json: bool,

    #[arg(short = 's', long = "split-empty-line", help = "Split by empty lines (default: 1 line)", value_name = "COUNT")]
    split_empty_line: Option<Option<usize>>,
}

#[tauri::command]
fn get_clipboard_data(state: State<AppState>) -> Result<AppData, String> {
    println!("get_clipboard_data called");
    let data = state.data.lock().unwrap();

    match &*data {
        Some(app_data) => {
            println!("Returning app_data with {} items", app_data.items.len());
            Ok(app_data.clone())
        }
        None => {
            println!("No data available in state");
            Err("No data available".to_string())
        }
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

    // CLI 引数を解析
    let args = Args::parse();
    let is_default_data = buffer.trim().starts_with('[');

    // モードと設定を決定
    let (mode, split_empty_line_count) = if args.multiline {
        ("multiline", 1)
    } else if let Some(count_opt) = args.split_empty_line {
        let count = count_opt.unwrap_or(1); // --split-empty-line または --split-empty-line=N
        ("split-empty-line", count)
    } else if args.json || is_default_data {
        ("json", 1)
    } else {
        ("normal", 1)
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
            // 指定された数の空行で分割
            let delimiter = "\n".repeat(split_empty_line_count + 1);
            buffer
                .split(&delimiter)
                .filter(|section| !section.trim().is_empty())
                .map(|section| {
                    ClipboardItem {
                        label: section.to_string(),
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
    let initial_data = match read_stdin_data() {
        Ok(data) => {
            println!("Successfully read stdin data: {} items", data.items.len());
            Some(data)
        }
        Err(e) => {
            eprintln!("Error reading stdin data: {}", e);
            None
        }
    };

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
