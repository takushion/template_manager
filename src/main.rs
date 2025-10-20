
use std::fs::{File, OpenOptions}; // ファイル操作を強化
use std::io::{self, BufReader, BufWriter, Stdout};
use serde::{Deserialize, Serialize}; // serde をインポート
use arboard::Clipboard;

use unicode_width::UnicodeWidthStr;
use textwrap::wrap;

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
    title_input: String,
    body_input: String,
    focused_input: InputFocus, 
    editing_index: Option<usize>,
}

enum Mode {
    Viewing,        // 通常の表示・スクロールモード
    ConfirmDelete,  // 削除確認プロンプト表示モード
    Editing,
}

enum InputFocus {
    Title,
    Body,
}

impl App {

    fn new(templates: Vec<Template>) -> App {
        let mut state = ListState::default();
        if !templates.is_empty() {
            state.select(Some(0));
        }
        App {
            templates,
            state,
            mode: Mode::Viewing,
            title_input: String::new(),
            body_input: String::new(),
            focused_input: InputFocus::Title, 
            editing_index: None,
        }
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
    pub fn enter_new_mode(&mut self) {
        self.editing_index = None;
        self.title_input = String::new();
        self.body_input = String::new();
        self.focused_input = InputFocus::Title;
        self.mode = Mode::Editing;
    }
    pub fn enter_edit_mode(&mut self) {
        if let Some(index) = self.state.selected() {
            if let Some(template) = self.templates.get(index) {
                self.editing_index = Some(index);
                // 選択中のテンプレートの内容を TextArea にロード
                self.title_input = template.title.clone();
                self.body_input = template.body.clone();
                self.focused_input = InputFocus::Title;
                self.mode = Mode::Editing;
            }
        }
    }
    pub fn exit_editing_mode(&mut self) {
        self.mode = Mode::Viewing;
    }
    pub fn save_template(&mut self) {
        let title = self.title_input.clone();
        let body = self.body_input.clone();

        if let Some(index) = self.editing_index {
            // 既存の編集
            if let Some(template) = self.templates.get_mut(index) {
                template.title = title;
                template.body = body;
            }
        } else {
            // 新規作成
            let new_template = Template { title, body };
            self.templates.push(new_template);
            self.state.select(Some(self.templates.len() - 1));
        }
        self.mode = Mode::Viewing;
    }
    pub fn switch_focus(&mut self) {
        match self.focused_input {
            InputFocus::Title => self.focused_input = InputFocus::Body,
            InputFocus::Body => self.focused_input = InputFocus::Title,
        }
    }
    pub fn input_char(&mut self, c: char) {
        match self.focused_input {
            InputFocus::Title => self.title_input.push(c),
            InputFocus::Body => self.body_input.push(c),
        }
    }
    
    /// アクティブな入力フィールドから文字を削除 (Backspace)
    pub fn input_backspace(&mut self) {
        match self.focused_input {
            InputFocus::Title => { self.title_input.pop(); },
            InputFocus::Body => { self.body_input.pop(); },
        }
    }
    
    /// 本文に改行を追加
    pub fn input_enter(&mut self) {
        if let InputFocus::Body = self.focused_input {
            self.body_input.push('\n');
        }
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
            ui(frame, app); 
        })?;

        // --- 2. イベントの処理 (キー入力) ---
        if event::poll(std::time::Duration::from_millis(100))? {
            let event = event::read()?;
                // app.mode に応じてキー操作を分岐
            
            match app.mode {
                Mode::Viewing => {
                    if let Event::Key(key) = event {
                        match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('j') | KeyCode::Down => app.next(),
                            KeyCode::Char('k') | KeyCode::Up => app.previous(),
                            KeyCode::Char('c') => {
                                if let Some(index) = app.state.selected() {
                                    if let Some(template) = app.templates.get(index) {
                                        clipboard.set_text(template.body.clone()).unwrap_or(());
                                    }
                                }
                            },
                            KeyCode::Char('d') => app.enter_delete_mode(),
                            KeyCode::Char('n') => app.enter_new_mode(), // (追加)
                            KeyCode::Char('e') => app.enter_edit_mode(), // (追加)
                            _ => {}
                        }
                    }
                },
                Mode::ConfirmDelete => {
                    if let Event::Key(key) = event {
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Char('Y') => app.delete_selected(),
                            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.exit_delete_mode(),
                            _ => {}
                        }
                    }
                },
                // (ここから追加)
                Mode::Editing => {
                    if let Event::Key(key) = event {
                        match key.code {
                            KeyCode::Esc => {
                                app.exit_editing_mode();
                            }
                            KeyCode::Tab => {
                                app.switch_focus();
                            }
                            // Ctrl + s で保存
                            KeyCode::Char('s') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                app.save_template();
                            }
                            // 文字入力
                            KeyCode::Char(c) => {
                                app.input_char(c);
                            }
                            // バックスペース
                            KeyCode::Backspace => {
                                app.input_backspace();
                            }
                            // Enter (本文のみ)
                            KeyCode::Enter => {
                                app.input_enter();
                            }
                            _ => {} // 矢印キーなどは無視
                        }
                    }

                }
            }
        }
    }
}
/// UIを描画する
/// (この関数がTUIの「見た目」を定義する)
fn ui<'a>(frame: &mut Frame, app: &mut App) {
    match app.mode {
        Mode::Viewing | Mode::ConfirmDelete => {
            // --- 1. 閲覧モードのメインUI ---
            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0)])
                .split(frame.area());
            
            let content_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(main_layout[1]);
            
            // (変更) ヘルプテキストに 'n' と 'e' を追加
            let title_text = format!(
                "テンプレートマネージャ | {} 件 | 'j/k'移動, 'c'コピー, 'd'削除, 'n'新規, 'e'編集, 'q'終了",
                app.templates.len()
            );
            let title = Paragraph::new(title_text)
                .style(Style::default().fg(Color::White).bg(Color::Blue));

            let items: Vec<ListItem> = app.templates.iter().map(|t| ListItem::new(Line::from(t.title.clone()))).collect();
            let templates_list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("テンプレート一覧"))
                .highlight_style(Style::default().bg(Color::LightGreen).fg(Color::Black).add_modifier(Modifier::BOLD));
            
            let selected_body = match app.state.selected() {
                Some(index) => app.templates.get(index).map_or("...", |t| t.body.as_str()),
                None => "（テンプレートが選択されていません）",
            };
            let template_body = Paragraph::new(selected_body)
                .block(Block::default().borders(Borders::ALL).title("本文"))
                .wrap(ratatui::widgets::Wrap { trim: false });

            frame.render_widget(title, main_layout[0]);
            frame.render_stateful_widget(templates_list, content_layout[0], &mut app.state);
            frame.render_widget(template_body, content_layout[1]);

            if let Mode::ConfirmDelete = app.mode {
                // (ここから元のポップアップロジックに戻す)
                if let Some(index) = app.state.selected() {
                    if let Some(template) = app.templates.get(index) {
                        // ポップアップ用のテキストを作成
                        let text = vec![
                            Line::from(Span::from(format!("'{}'を本当に削除しますか？", template.title))), 
                            Line::from(Span::from("y: はい    n: いいえ")),
                        ];
                        draw_popup(frame, "本当に削除しますか？(y/n)", text);
                    }
                }
                // (ここまで)
            }
        }
        // (ここから追加)
        Mode::Editing => {
            // --- 3. 編集モードのUI ---
            let edit_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // タイトル入力 (3行)
                    Constraint::Min(0),    // 本文入力 (残り)
                    Constraint::Length(1), // ヘルプ
                ])
                .split(frame.area());
            
            
            let focused_style = Style::default().fg(Color::Yellow);
            let unfocused_style = Style::default().fg(Color::DarkGray);

            let title_block = Block::default()
                .borders(Borders::ALL)
                .title("タイトル (編集中)")
                .border_style(match app.focused_input {
                    InputFocus::Title => focused_style,
                    _ => unfocused_style,
                });

            let body_block = Block::default()
                .borders(Borders::ALL)
                .title("本文 (編集中)")
                .border_style(match app.focused_input {
                    InputFocus::Body => focused_style,
                    _ => unfocused_style,
                });

            // --- Paragraph ウィジェットの作成 ---
            // String を Paragraph で包む
            let title_input_widget = Paragraph::new(app.title_input.as_str())
                .block(title_block); // 作成した Block を設定

            let body_input_widget = Paragraph::new(app.body_input.as_str())
                .block(body_block) // 作成した Block を設定
                .wrap(ratatui::widgets::Wrap { trim: false }); // 折り返し

            let help = Paragraph::new("'Tab'でフォーカス切替, 'Ctrl+s'で保存, 'Esc'でキャンセル")
                .style(Style::default().fg(Color::Gray));

            // --- ウィジェットの描画 ---
            // 作成した Paragraph ウィジェットを描画する
            frame.render_widget(title_input_widget, edit_layout[0]);
            frame.render_widget(body_input_widget, edit_layout[1]);
            frame.render_widget(help, edit_layout[2]);
            
            match app.focused_input {
                InputFocus::Title => {
                    let display_width = UnicodeWidthStr::width(app.title_input.as_str());
                    let cursor_x = edit_layout[0].x + 1 + display_width as u16;
                    let cursor_y = edit_layout[0].y + 1;
                    // (変更) 引数をタプル (x, y) で囲む
                    frame.set_cursor_position((cursor_x, cursor_y));
                }
                InputFocus::Body => {
                    let text_area_width = edit_layout[1].width.saturating_sub(2) as usize;
                    if text_area_width == 0 { return; }

                    // textwrap::wrap を使って、現在のテキストがどのように折り返されるかを取得
                    let wrapped_lines = wrap(&app.body_input, text_area_width);

                    // 現在のY座標は、折り返された後の行数
                    let cursor_y = wrapped_lines.len().saturating_sub(1);

                    // 現在のX座標は、最後の行の表示幅
                    let cursor_x = wrapped_lines.last().map_or(0, |line| UnicodeWidthStr::width(line.as_ref()));

                    frame.set_cursor_position((
                        edit_layout[1].x + 1 + cursor_x as u16,
                        edit_layout[1].y + 1 + cursor_y as u16,
                    ));
                }
            }
        }
        // (ここまで追加)
    }
}
fn draw_popup(frame: &mut Frame, title: &str, text: Vec<Line>) {
    // ポップアップのサイズを定義 (ここでは固定)
    let area = centered_rect(50, 20, frame.area());

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