use serde::{Deserialize, Serialize};
use std::io::{self, IsTerminal, Read};
use std::sync::Mutex;
use tauri::{Manager, State, Theme};
use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ThemeArg {
    /// Follow the OS setting
    Auto,
    /// Always use the light theme
    Light,
    /// Always use the dark theme
    Dark,
}

impl ThemeArg {
    fn as_str(&self) -> &'static str {
        match self {
            ThemeArg::Auto => "auto",
            ThemeArg::Light => "light",
            ThemeArg::Dark => "dark",
        }
    }

    /// ウィンドウ (タイトルバー) に適用するテーマ。Auto は None で OS 追従。
    fn window_theme(&self) -> Option<Theme> {
        match self {
            ThemeArg::Auto => None,
            ThemeArg::Light => Some(Theme::Light),
            ThemeArg::Dark => Some(Theme::Dark),
        }
    }
}

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
    pub theme: String,
}

pub struct AppState {
    pub data: Mutex<Option<AppData>>,
}

#[derive(Parser)]
#[command(name = "clipboard-palette")]
#[command(version)]
#[command(about = "Show clipboard copy buttons for text read from standard input.")]
#[command(long_about = "Show clipboard copy buttons for text read from standard input.

Text is read from standard input, split into items according to the selected
mode, and each item is shown as a button. Clicking a button copies that item
to the clipboard. With no options, the entire input becomes a single item,
with leading and trailing whitespace stripped.

If stdin is a terminal (no pipe), or the input is empty or whitespace only,
sample data is shown instead so you can try the app out.")]
#[command(after_long_help = r#"MODES:
  Exactly one mode applies. If several are given, the first match in this
  list wins: --multiline, --split-empty-line, --json.

  normal (default)     Copy the entire input as one item, trimmed.
  -m, --multiline      One item per line. Blank lines are dropped.
  -s, --split-empty-line[=COUNT]
                       Split into sections at COUNT consecutive empty lines
                       (COUNT defaults to 1). The separator is a literal run
                       of COUNT+1 newlines, so CRLF input and lines holding
                       only spaces do not separate sections, and any extra
                       newlines stay at the head of the next section.
                       COUNT=0 splits on every newline.
  -j, --json           Parse the input as a JSON array of objects. JSON is
                       never auto-detected, so this flag is required.

JSON FORMAT:
  Each element needs a "label" and a "text" field. "label" is shown on the
  button, "text" is what gets copied when the button is clicked.

    [{"label": "Button Label", "text": "Text to copy"}, ...]

EXAMPLES:
  echo "Hello, World!" | clipboard-palette
  printf 'first\nsecond\n' | clipboard-palette --multiline
  printf 'a\n\n\nb\n' | clipboard-palette --split-empty-line=2
  echo '[{"label":"Greeting","text":"Hello"}]' | clipboard-palette --json
  pbpaste | clipboard-palette -m
  echo "Hello, World!" | clipboard-palette --theme=dark

THEME:
  The window content and the title bar follow the OS setting by default.
  --theme=light or --theme=dark forces one of them instead."#)]
#[command(after_help = "Run with --help for modes, JSON format and examples.")]
struct Args {
    /// Show one button per line. Blank lines are dropped
    #[arg(short = 'm', long = "multiline")]
    multiline: bool,

    /// Parse the input as a JSON array of {"label", "text"} objects
    #[arg(short = 'j', long = "json")]
    json: bool,

    /// Split the input into sections at COUNT consecutive empty lines [default: 1]
    #[arg(short = 's', long = "split-empty-line", value_name = "COUNT")]
    split_empty_line: Option<Option<usize>>,

    /// Force a color theme instead of following the OS setting
    #[arg(
        long = "theme",
        value_enum,
        default_value_t = ThemeArg::Auto,
        value_name = "THEME"
    )]
    theme: ThemeArg,
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

fn default_data_buffer() -> String {
    r#"[
    {"label": "ラベル1", "text": "テキスト1"},
    {"label": "ラベル2", "text": "テキスト2"}
]"#.to_string()
}

fn read_stdin_data(args: &Args) -> Result<AppData, String> {
    // TTY (ターミナル直接起動) ならstdinを読まずデフォルトデータを使用
    // is_default_data はサンプルデータで代替したかを表す
    let (buffer, is_default_data) = if io::stdin().is_terminal() {
        println!("stdin is a terminal, using default data");
        (default_data_buffer(), true)
    } else {
        // パイプ経由の場合のみstdinを読み取る
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("Failed to read from stdin: {}", e))?;
        println!("Input data received ({} bytes)", buf.len());
        let empty = buf.trim().is_empty();
        if empty {
            println!("Empty input detected, using default data");
            buf = default_data_buffer();
        }
        (buf, empty)
    };

    // モードと設定を決定
    // 先に一致したものが優先される (multiline > split-empty-line > json)
    let (mode, split_empty_line_count) = if args.multiline {
        ("multiline", 1)
    } else if let Some(count_opt) = args.split_empty_line {
        let count = count_opt.unwrap_or(1); // --split-empty-line または --split-empty-line=N
        ("split-empty-line", count)
    } else if args.json || is_default_data {
        // サンプルデータは JSON 形式なので JSON モードで解析する。
        // 入力内容による JSON の自動判定は行わない (--json の明示が必要)
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
        is_default_data,
        theme: args.theme.as_str().to_string(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // --help 時に stdin をブロックしないよう、先に引数を解析
    let args = Args::parse();
    // タイトルバーを含むウィンドウのテーマ (auto の場合は OS 追従)
    let window_theme = args.theme.window_theme();
    // 起動時に標準入力を読み取る
    let initial_data = match read_stdin_data(&args) {
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
        .setup(move |app| {
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_cli::init())?;

            // アプリケーションステートを設定
            app.manage(AppState {
                data: Mutex::new(initial_data),
            });

            // ウィンドウ (タイトルバー) のテーマを適用
            if let Some(theme) = window_theme {
                if let Some(window) = app.get_webview_window("main") {
                    if let Err(e) = window.set_theme(Some(theme)) {
                        eprintln!("Failed to set window theme: {}", e);
                    }
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_clipboard_data])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
