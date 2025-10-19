
use std::fs::{File, OpenOptions}; // ファイル操作を強化
use std::io::{self, BufReader, BufWriter, Stdout};
use std::path::PathBuf;
use serde::{Deserialize, Serialize}; // serde をインポート

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, List, ListItem,},
};


const JSON_FILE: &str = "templates.json";

/// テンプレートのデータ構造
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Template {
    title: String,
    body: String,
}

/// アプリケーションの状態を保持する構造体
struct App {
    templates: Vec<Template>,
}

impl App {
    /// 新しいAppインスタンスを作成
    fn new(templates: Vec<Template>) -> App {
        App { templates }
    }
}


fn main() -> Result<(), Box<dyn std::error::Error>>{
    let templates = load_templates(JSON_FILE)?;
    let mut app = App::new(templates);

    // --- 2. TUIのセットアップ ---
    let mut terminal = setup_terminal()?;

    // --- 3. TUIアプリケーションの実行 ---
    // App の「可変」参照をTUIに渡す
    run_app(&mut terminal, &mut app)?;

    // --- 4. TUIの後片付け ---
    restore_terminal(&mut terminal)?;

    // --- 5. データの保存 ---
    // アプリケーションの状態をJSONファイルに保存
    save_templates(JSON_FILE, &app)?;

    Ok(())
}

// (ここからJSON I/O関数を新設)
/// JSONファイルからテンプレートをロードする
fn load_templates(filepath: &str) -> Result<Vec<Template>, Box<dyn std::error::Error>> {
    // ファイルが存在しない場合は、空のリストを返す (初回起動時など)
    let file = match File::open(filepath) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Ok(Vec::new()); // 空のVecを返す
        }
        Err(e) => {
            return Err(Box::new(e)); // その他のエラーは返す
        }
    };

    let reader = BufReader::new(file);
    // JSONをパースして Vec<Template> に変換
    let templates = serde_json::from_reader(reader)?;

    Ok(templates)
}

/// テンプレートをJSONファイルに保存する
fn save_templates(filepath: &str, app: &App) -> Result<(), Box<dyn std::error::Error>> {
    // ファイルを書き込みモード (または新規作成) で開く
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true) // 既存の内容を上書き
        .open(filepath)?;

    let writer = BufWriter::new(file);
    // AppのテンプレートリストをJSONにシリアライズして書き込む
    serde_json::to_writer_pretty(writer, &app.templates)?;

    Ok(())
}


// (ここからTUI関連の関数をすべて追加)

/// TUIのセットアップ
/// (ターミナルをTUI描画用に初期化する)
fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, Box<dyn std::error::Error>> {
    let mut stdout = io::stdout();
    enable_raw_mode()?; // Rawモード: キー入力を即時受け取る (Enter不要)
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?; // 
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

/// TUIの後片付け
/// (ターミナルの状態を元に戻す)
fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// TUIアプリケーションのメインループ
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App, // (変更) 可変参照 (&mut App) を受け取る
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // --- 1. UIの描画 ---
        terminal.draw(|frame| {
            ui(frame, app); // (変更) Appの参照をui関数に渡す
        })?;

        // --- 2. イベントの処理 (キー入力) ---
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    return Ok(());
                }
            }
        }
    }
}
/// UIを描画する
/// (この関数がTUIの「見た目」を定義する)
fn ui(frame: &mut Frame, app: &App) {
    // 画面全体をレイアウト
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(frame.size());

    let title_text = format!(
        "アノテーション検索TUI | {} 件ヒット | 'q' で終了",
        app.templates.len() // Appから件数を取得
    );
    
    // 2-1. タイトル
    let title_text = format!(
        "テンプレートマネージャ | {} 件登録 | 'q' で終了",
        app.templates.len() // (変更) app.templates から件数を取得
    );
    let title = Paragraph::new(title_text)
        .style(Style::default().fg(Color::White).bg(Color::Blue));

    // 2-2. テンプレートのタイトルリスト
    // (変更) Vec<Template> から Vec<ListItem> に変換
    let items: Vec<ListItem> = app
        .templates
        .iter()
        .map(|t| {
            // (変更) テンプレートの「タイトル」を表示
            ListItem::new(Line::from(t.title.clone()))
        })
        .collect();

    // テンプレートのリストウィジェットを作成
    let templates_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("テンプレート一覧"));

    // --- 3. ウィジェットの描画 ---
    frame.render_widget(title, main_layout[0]);
    frame.render_widget(templates_list, main_layout[1]); // (変更) matches_list から名前変更
}