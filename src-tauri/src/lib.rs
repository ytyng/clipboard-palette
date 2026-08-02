use serde::{Deserialize, Serialize};
use std::io::{self, IsTerminal, Read};
use std::sync::Mutex;
use tauri::{Manager, State, Theme, WebviewWindowBuilder};
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

    /// Theme applied to the window (and its title bar). Auto is None, i.e. follow the OS.
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
    // When stdin is a TTY (launched straight from a terminal) do not read it and
    // use the sample data instead. is_default_data records that substitution
    let (buffer, is_default_data) = if io::stdin().is_terminal() {
        println!("stdin is a terminal, using default data");
        (default_data_buffer(), true)
    } else {
        // Only read stdin when it comes through a pipe
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

    // Decide the mode and its settings.
    // The first match wins (multiline > split-empty-line > json)
    let (mode, split_empty_line_count) = if args.multiline {
        ("multiline", 1)
    } else if let Some(count_opt) = args.split_empty_line {
        let count = count_opt.unwrap_or(1); // --split-empty-line or --split-empty-line=N
        ("split-empty-line", count)
    } else if args.json || is_default_data {
        // The sample data is JSON, so parse it in JSON mode.
        // JSON is never auto-detected from the input (--json is required)
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
            // Split at the given number of empty lines
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
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Parse the arguments first so that --help does not block on stdin
    let args = Args::parse();
    // Theme of the window including its title bar (auto follows the OS)
    let window_theme = args.theme.window_theme();
    // Theme name handed to the pre-paint script
    let theme_name = args.theme.as_str();
    // Read standard input at startup
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

            // Set up the application state
            app.manage(AppState {
                data: Mutex::new(initial_data),
            });

            // The window is declared with create: false in tauri.conf.json and is built
            // here instead, so that an initialization script can inject the --theme value
            // into the page (src/app.html reads it) before the first paint
            let window_config = app
                .config()
                .app
                .windows
                .iter()
                .find(|w| w.label == "main")
                .cloned()
                .ok_or("window config \"main\" not found")?;
            let init_script = format!(
                "window.__CLIPBOARD_PALETTE_THEME__ = {};",
                serde_json::to_string(theme_name)?
            );
            // The theme goes on the builder rather than being applied afterwards, so
            // the title bar never paints with the OS theme first. None follows the OS
            WebviewWindowBuilder::from_config(app.handle(), &window_config)?
                .initialization_script(init_script)
                .theme(window_theme)
                .build()?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_clipboard_data])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
