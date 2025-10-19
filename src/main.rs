
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
    widgets::{Block, Borders, Paragraph, List, ListItem, ListState},
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
    state: ListState,
}

impl App {
    /// 新しいAppインスタンスを作成
    fn new(templates: Vec<Template>) -> App {
        let mut state = ListState::default();
        if !templates.is_empty() {
            state.select(Some(0)); // 最初の項目 (インデックス0) を選択状態にする
        }
        App { templates, state } // state を初期化
    }


    pub fn next(&mut self) {
        if self.templates.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.templates.len() - 1 {
                    0 // リストの最後に達したら最初に戻る (ラップアラウンド)
                } else {
                    i + 1
                }
            }
            None => 0, // 何も選択されていなければ0を選択
        };
        self.state.select(Some(i));
    }

    /// リストで前の項目を選択
    pub fn previous(&mut self) {
        if self.templates.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.templates.len() - 1 // リストの最初に達したら最後に戻る (ラップアラウンド)
                } else {
                    i - 1
                }
            }
            None => 0, // 何も選択されていなければ0を選択
        };
        self.state.select(Some(i));
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
                // (変更) キー入力処理
                match key.code {
                    KeyCode::Char('q') => {
                        // 'q' キーが押されたら終了
                        return Ok(());
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        // 'j' または 下矢印で次へ
                        app.next();
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        // 'k' または 上矢印で前へ
                        app.previous();
                    }
                    _ => {} // 他のキーは無視
                }
            }
        }
    }
}
/// UIを描画する
/// (この関数がTUIの「見た目」を定義する)
fn ui(frame: &mut Frame, app: &mut App) {
    // 画面全体をレイアウト
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(frame.size());

    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // 左ペイン (タイトルリスト)
            Constraint::Percentage(60), // 右ペイン (本文表示)
        ])
        .split(main_layout[1]); // main_layout[1] を分割
    // (ここまで変更)

    // --- 2. ウィジェットの作成 ---

    // 2-1. タイトルバー
    let title_text = format!(
        "テンプレートマネージャ | {} 件登録 | 'j/k'で移動, 'q'で終了",
        app.templates.len()
    );
    let title = Paragraph::new(title_text)
        .style(Style::default().fg(Color::White).bg(Color::Blue));

    // 2-2. テンプレートのタイトルリスト (左ペイン)
    let items: Vec<ListItem> = app
        .templates
        .iter()
        .map(|t| ListItem::new(Line::from(t.title.clone())))
        .collect();

    let templates_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("テンプレート一覧"))
        .highlight_style( // (追加) 選択ハイライトのスタイル
            Style::default()
                .bg(Color::LightGreen) // 背景色
                .fg(Color::Black)      // 文字色
                .add_modifier(Modifier::BOLD), // 太字
        );

    // (ここから追加)
    // 2-3. 選択中のテンプレート本文 (右ペイン)
    // 現在選択中のインデックスを取得
    let selected_body = match app.state.selected() {
        Some(index) => {
            // インデックスが範囲内なら、そのテンプレートの本文
            app.templates.get(index).map_or(
                "（テンプレートが選択されていません）",
                |t| t.body.as_str(), // .as_str() で &str を取得
            )
        }
        None => "（テンプレートが選択されていません）", // 何も選択されていない場合
    };

    let template_body = Paragraph::new(selected_body)
        .block(Block::default().borders(Borders::ALL).title("本文"))
        .wrap(ratatui::widgets::Wrap { trim: false }); // (追加) 自動で折り返し
    // (ここまで追加)

    // --- 3. ウィジェットの描画 ---
    frame.render_widget(title, main_layout[0]);
    
    // (変更) 左ペイン (content_layout[0]) に「ステートフル」ウィジェットを描画
    frame.render_stateful_widget(templates_list, content_layout[0], &mut app.state);
    
    // (変更) 右ペイン (content_layout[1]) に本文を描画
    frame.render_widget(template_body, content_layout[1]);
}