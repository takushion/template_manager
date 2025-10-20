
use std::fs::{File, OpenOptions}; // ファイル操作を強化
use std::io::{self, BufReader, BufWriter, Stdout};
use serde::{Deserialize, Serialize}; // serde をインポート
use arboard::Clipboard;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, List, ListItem, ListState, Clear},
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
    mode: Mode,
}

enum Mode {
    Viewing,        // 通常の表示・スクロールモード
    ConfirmDelete,  // 削除確認プロンプト表示モード
}

impl App {
    /// 新しいAppインスタンスを作成
    fn new(templates: Vec<Template>) -> App {
        let mut state = ListState::default();
        if !templates.is_empty() {
            state.select(Some(0)); // 最初の項目 (インデックス0) を選択状態にする
        }
        App { templates, state, mode: Mode::Viewing, } // state を初期化
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

    pub fn enter_delete_mode(&mut self) {
        // 何も選択されていない場合は何もしない
        if self.state.selected().is_some() {
            self.mode = Mode::ConfirmDelete;
        }
    }

    /// 閲覧モードに戻る
    pub fn exit_delete_mode(&mut self) {
        self.mode = Mode::Viewing;
    }

    /// 選択中のテンプレートを削除
    pub fn delete_selected(&mut self) {
        if let Some(selected_index) = self.state.selected() {
            // 1. Vecから削除
            self.templates.remove(selected_index);

            // 2. ListState を調整
            if self.templates.is_empty() {
                // リストが空になった
                self.state.select(None);
            } else if selected_index >= self.templates.len() {
                // 削除したのが最後の要素だった場合、新しい最後の要素を選択
                self.state.select(Some(self.templates.len() - 1));
            }
            // (それ以外の場合は、ListState は自動的に次の要素を指す (または同じインデックスを維持) ので調整不要)
        }
        self.mode = Mode::Viewing; // 閲覧モードに戻る
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

    let mut clipboard = Clipboard::new()?;
    loop {
        // --- 1. UIの描画 ---
        terminal.draw(|frame| {
            ui(frame, app); // (変更) Appの参照をui関数に渡す
        })?;

        // --- 2. イベントの処理 (キー入力) ---
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // app.mode に応じてキー操作を分岐
                match app.mode {
                    // --- 通常モードの操作 ---
                    Mode::Viewing => match key.code {
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.next();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.previous();
                        }
                        KeyCode::Char('c') => {
                            if let Some(index) = app.state.selected() {
                                if let Some(template) = app.templates.get(index) {
                                    clipboard.set_text(template.body.clone())?;
                                }
                            }
                        }
                        KeyCode::Char('d') => {
                            app.enter_delete_mode();
                        }
                        _ => {}
                    },
                    Mode::ConfirmDelete => match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            app.delete_selected();
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            app.exit_delete_mode();
                        }
                        _ => {}
                    }, 
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
        "テンプレートマネージャ | {} 件 | 'j/k'で移動, 'c'でコピー, 'q'で終了",
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

    if let Mode::ConfirmDelete = app.mode {
        if let Some(index) = app.state.selected() {
            if let Some(template) = app.templates.get(index) {
                // ポップアップ用のテキストを作成
                let text = vec![
                    Line::from(Span::styled("(y) はい / (n) いいえ", Style::default().fg(Color::Gray))),
                ];
                // ポップアップを描画
                draw_popup(frame, "本当に削除しますか?", text);
            }
        }
    }
}

fn draw_popup(frame: &mut Frame, title: &str, text: Vec<Line>) {
    // ポップアップのサイズを定義 (ここでは固定)
    let area = centered_rect(50, 20, frame.size());

    let popup_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray)); // ポップアップの背景色

    let popup_text = Paragraph::new(text)
        .block(popup_block)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .wrap(ratatui::widgets::Wrap { trim: true });

    // `Clear` ウィジェットで、ポップアップの描画範囲を一度クリアする
    frame.render_widget(Clear, area); 
    // ポップアップを描画
    frame.render_widget(popup_text, area); 
}

/// 画面中央に指定したサイズの矩形(Rect)を計算するヘルパー関数
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1] // [1] が中央のエリア
}