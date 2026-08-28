mod lifecycle;

use serde::{Deserialize, Serialize};
use std::io::{self, IsTerminal, Read};
use std::sync::Mutex;
use tauri::{App, Manager, RunEvent, State, Theme, WebviewWindowBuilder};
use clap::{Parser, ValueEnum};

/// Label of the only window, as declared in tauri.conf.json
const MAIN_WINDOW_LABEL: &str = "main";

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

    let (mode, split_empty_line_count) = select_mode(args, is_default_data);
    let items = build_items(&buffer, mode, split_empty_line_count)?;

    println!("Processing mode: {}", mode);
    println!("Created {} clipboard items", items.len());

    Ok(AppData {
        items,
        mode: mode.to_string(),
        is_default_data,
    })
}

/// Decide the mode and its settings.
///
/// The first match wins (multiline > split-empty-line > json), which is what
/// the help text promises. Kept apart from the stdin read so it can be tested
/// without a pipe.
fn select_mode(args: &Args, is_default_data: bool) -> (&'static str, usize) {
    if args.multiline {
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
    }
}

/// Split the input into the buttons the window shows.
///
/// Pure: this is what turns the piped text into what the user clicks, so it is
/// the part worth pinning down with tests.
fn build_items(
    buffer: &str,
    mode: &str,
    split_empty_line_count: usize,
) -> Result<Vec<ClipboardItem>, String> {
    let items = match mode {
        "multiline" => buffer
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| ClipboardItem {
                label: line.to_string(),
                text: line.to_string(),
            })
            .collect(),
        "split-empty-line" => {
            // Split at the given number of empty lines
            let delimiter = "\n".repeat(split_empty_line_count + 1);
            buffer
                .split(&delimiter)
                .filter(|section| !section.trim().is_empty())
                .map(|section| ClipboardItem {
                    label: section.to_string(),
                    text: section.to_string(),
                })
                .collect()
        }
        "json" => serde_json::from_str::<Vec<ClipboardItem>>(buffer)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?,
        _ => {
            // normal mode
            vec![ClipboardItem {
                label: buffer.trim().to_string(),
                text: buffer.trim().to_string(),
            }]
        }
    };
    Ok(items)
}

/// Build the main window.
///
/// The window is declared with create: false in tauri.conf.json and is built
/// here instead, so that an initialization script can inject the --theme value
/// into the page (src/app.html reads it) before the first paint
fn build_main_window(
    app: &App,
    theme_name: &str,
    window_theme: Option<Theme>,
) -> Result<(), Box<dyn std::error::Error>> {
    let window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == MAIN_WINDOW_LABEL)
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
    lifecycle::log_window_created(MAIN_WINDOW_LABEL);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Parse the arguments first so that --help does not block on stdin
    let args = Args::parse();
    // Install the failure reporting first, so that even a panic in the startup
    // reporting below shows up on stdout
    lifecycle::install_panic_logger();
    lifecycle::install_signal_logger();
    // Report the process id and any leftover instance before anything else, so
    // the caller can see what was already running when this one started
    lifecycle::report_instances();
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

    let app = tauri::Builder::default()
        .setup(move |app| {
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_cli::init())?;

            // Set up the application state
            app.manage(AppState {
                data: Mutex::new(initial_data),
            });

            build_main_window(app, theme_name, window_theme).inspect_err(|e| {
                // Log before propagating: the failure that follows only reaches
                // stderr, and the caller decides the outcome from stdout
                lifecycle::log_startup_failed(&e.to_string());
            })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_clipboard_data])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // Run the event loop by hand rather than with Builder::run, so the window
    // state and the exit reason can be logged
    app.run(|app_handle, event| match event {
        // The window is on screen by the time the event loop is ready, so this
        // is the first point where visibility means anything
        RunEvent::Ready => lifecycle::log_window_state(app_handle, MAIN_WINDOW_LABEL),
        RunEvent::WindowEvent { label, event, .. } => lifecycle::log_window_event(&label, &event),
        RunEvent::ExitRequested { code, .. } => lifecycle::log_exit_requested(code),
        RunEvent::Exit => lifecycle::log_exit(),
        _ => {}
    });
}

#[cfg(test)]
mod tests {
    use super::{build_items, default_data_buffer, select_mode, Args, ThemeArg};
    use clap::Parser;

    /// Parse arguments the way the binary does, without the binary.
    fn args_from(argv: &[&str]) -> Args {
        let mut full = vec!["clipboard-palette"];
        full.extend_from_slice(argv);
        Args::parse_from(full)
    }

    #[test]
    fn no_option_reads_the_input_as_one_item() {
        let (mode, count) = select_mode(&args_from(&[]), false);
        assert_eq!((mode, count), ("normal", 1));
    }

    #[test]
    fn multiline_wins_over_the_other_modes() {
        // The help text promises multiline > split-empty-line > json, so the
        // order of the branches is part of the contract
        let args = args_from(&["--multiline", "--split-empty-line=3", "--json"]);
        assert_eq!(select_mode(&args, false).0, "multiline");
    }

    #[test]
    fn split_empty_line_wins_over_json() {
        let args = args_from(&["--split-empty-line", "--json"]);
        assert_eq!(select_mode(&args, false), ("split-empty-line", 1));
    }

    #[test]
    fn split_empty_line_takes_an_optional_count() {
        assert_eq!(
            select_mode(&args_from(&["--split-empty-line=2"]), false),
            ("split-empty-line", 2)
        );
        assert_eq!(
            select_mode(&args_from(&["-s", "0"]), false),
            ("split-empty-line", 0)
        );
    }

    #[test]
    fn json_needs_its_flag() {
        // JSON is never auto-detected, so the flag is the only way to ask for
        // it with real input
        assert_eq!(select_mode(&args_from(&["--json"]), false), ("json", 1));
        assert_eq!(select_mode(&args_from(&["-j"]), false), ("json", 1));
    }

    #[test]
    fn the_sample_data_is_read_as_json_without_the_flag() {
        // JSON is never auto-detected from real input, but the sample data the
        // app falls back to is JSON, so it has to be parsed as such
        assert_eq!(select_mode(&args_from(&[]), true).0, "json");
        assert_eq!(select_mode(&args_from(&[]), false).0, "normal");
    }

    #[test]
    fn multiline_drops_blank_lines() {
        let items = build_items("first\n\n  \nsecond\n", "multiline", 1).unwrap();
        let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, vec!["first", "second"]);
        assert_eq!(items[0].text, "first");
    }

    #[test]
    fn normal_mode_trims_the_whole_input() {
        let items = build_items("  hello\nworld  \n", "normal", 1).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].text, "hello\nworld");
    }

    #[test]
    fn split_empty_line_splits_on_the_requested_run_of_newlines() {
        let items = build_items("a\n\n\nb\n", "split-empty-line", 2).unwrap();
        let texts: Vec<_> = items.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(texts, vec!["a", "b\n"]);
    }

    #[test]
    fn split_empty_line_keeps_shorter_runs_inside_a_section() {
        // One blank line is not a separator when a count of 2 was asked for
        let items = build_items("a\n\nb\n\n\nc", "split-empty-line", 2).unwrap();
        let texts: Vec<_> = items.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(texts, vec!["a\n\nb", "c"]);
    }

    #[test]
    fn split_empty_line_does_not_treat_crlf_or_spaces_as_a_separator() {
        // The separator is a literal run of newlines, as the help text says.
        // A line holding only spaces, and CRLF input, stay inside the section
        let items = build_items("a\r\n\r\nb", "split-empty-line", 1).unwrap();
        assert_eq!(items.len(), 1, "CRLF must not separate sections");
        let items = build_items("a\n \nb", "split-empty-line", 1).unwrap();
        assert_eq!(items.len(), 1, "a line of spaces must not separate sections");
    }

    #[test]
    fn split_empty_line_with_zero_splits_every_line() {
        let items = build_items("a\nb\n\nc", "split-empty-line", 0).unwrap();
        let texts: Vec<_> = items.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(texts, vec!["a", "b", "c"]);
    }

    #[test]
    fn json_mode_reads_label_and_text() {
        let items =
            build_items(r#"[{"label": "L", "text": "T"}]"#, "json", 1).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "L");
        assert_eq!(items[0].text, "T");
    }

    #[test]
    fn json_mode_reports_malformed_input() {
        // Reported rather than shown as a single button holding the raw JSON
        let err = build_items("not json", "json", 1).unwrap_err();
        assert!(err.starts_with("Failed to parse JSON"), "{}", err);
    }

    #[test]
    fn the_sample_data_parses_in_json_mode() {
        // The fallback shown on a TTY or on empty input has to survive the
        // mode that select_mode picks for it
        let buffer = default_data_buffer();
        let items = build_items(&buffer, "json", 1).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn theme_defaults_to_following_the_os() {
        let args = args_from(&[]);
        assert_eq!(args.theme.as_str(), "auto");
        assert!(args.theme.window_theme().is_none());
    }

    #[test]
    fn theme_can_be_forced() {
        assert_eq!(args_from(&["--theme=dark"]).theme.as_str(), "dark");
        assert_eq!(args_from(&["--theme", "light"]).theme.as_str(), "light");
        assert!(matches!(
            args_from(&["--theme=dark"]).theme,
            ThemeArg::Dark
        ));
        assert!(args_from(&["--theme=dark"]).theme.window_theme().is_some());
    }

    #[test]
    fn an_unknown_theme_is_rejected() {
        assert!(Args::try_parse_from(["clipboard-palette", "--theme=neon"]).is_err());
    }
}
