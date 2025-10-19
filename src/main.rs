
use std::fs::File; // ファイルを扱うためのモジュール
use std::io::{self,BufRead, BufReader, Stdout};
use std::path::PathBuf;
use clap::Parser;
//use walkdir::WalkDir;
use ignore::Walk;

use rayon::prelude::*;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, List, ListItem,},
};

#[derive(Debug)] // デバッグ表示できるようにする
struct Match {
    filepath: PathBuf,
    line_num: usize,
    content: String,
}

#[derive(Parser,Debug)]
#[command(version, about = "指定されたファイルからキーワードを検索します")]

struct Cli {

    /// 検索対象のファイルパス
    #[arg(required = true)]
    target_dir: PathBuf,

    #[arg(short = 'i', long = "ignore-case")]
    ignore_case: bool,
}

const KEYWORDS: [&str; 2] = ["TODO", "FIXME"];

struct App {
    matches: Vec<Match>,
}

impl App {
    /// 新しいAppインスタンスを作成
    fn new(matches: Vec<Match>) -> App {
        App { matches }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>>{
    let cli = Cli::parse();

    let files_to_search: Vec<PathBuf> = Walk::new(&cli.target_dir)
        .filter_map(|result| match result {
            Ok(entry) => {
                if let Some(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        return Some(entry.path().to_path_buf());
                    }
                }
                None
            }
            Err(e) => {
                eprintln!("警告: ファイル探索中にエラーが発生しました: {}", e);
                None
            }
        })
        .collect();
    

    let all_matches: Vec<Match> = files_to_search
        .par_iter()
        .filter_map(|filepath| {
            match find_matches(filepath, &KEYWORDS, cli.ignore_case) {
                Ok(matches) => Some(matches), // 成功したら Some(Vec<Match>)
                Err(e) => {
                    eprintln!("警告: ファイル {} の処理中にエラー: {}", filepath.display(), e);
                    None // 失敗したら None (無視)
                }
            }
        })
        .flatten().collect();

    let app = App::new(all_matches);
    
    // --- 3. TUIのセットアップ ---
    let mut terminal = setup_terminal()?;

    // --- 4. TUIアプリケーションの実行 ---
    // 検索結果をTUIに渡す
    run_app(&mut terminal, app)?;

    // --- 5. TUIの後片付け ---
    restore_terminal(&mut terminal)?;

    Ok(())
}

fn find_matches(filepath: &PathBuf, keywords: &[&str], ignore_case: bool) -> Result<Vec<Match>, Box<dyn std::error::Error>> {


    let file = File::open(filepath)?; // ?演算子でエラーを自動的に呼び出し元に返す
    let reader = BufReader::new(file);

    let mut results: Vec<Match> = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        // .map_err() でエラーの種類を変換
        let line_content = line.map_err(|e| format!("行の読み込みに失敗しました: {}", e))?;


        for kw in keywords {
            let found = if ignore_case {
                // 検索キーワード(kw)も小文字に変換
                line_content.to_lowercase().contains(&kw.to_lowercase())
            } else {
                line_content.contains(kw)
            };

            if found {
                let m = Match {
                    filepath: filepath.clone(),
                    line_num: index + 1,
                    content: line_content.trim().to_string(),
                };
                results.push(m);
                break; 
            }
        }
    }

    Ok(results)

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
    app: App, // 検索結果を受け取る (今回はまだ使わない)
) -> Result<(), Box<dyn std::error::Error>> {
    
    // `loop` でTUIのメインループを開始
    loop {
        // --- 1. UIの描画 ---
        // `terminal.draw()` の中(クロージャ)に描画処理を書く
        terminal.draw(|frame| {
            // (frame: &mut Frame)
            ui(frame, &app);
        })?;

        // --- 2. イベントの処理 (キー入力) ---
        // 100ミリ秒待機してキー入力があるかチェック
        if event::poll(std::time::Duration::from_millis(100))? {
            // キー入力があった場合
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    // 'q' キーが押されたらループを抜けてプログラム終了
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
        app.matches.len() // Appから件数を取得
    );
    let title = Paragraph::new(title_text)
        .style(Style::default().fg(Color::White).bg(Color::Blue)); // スタイル(白文字/青背景)

    // 2-2. 検索結果リスト
    // Vec<Match> を Vec<ListItem> に変換
    let items: Vec<ListItem> = app
        .matches
        .iter()
        .map(|m| {
            // 1行のテキストを作成
            let line_text = format!(
                "{}:{}: {}",
                m.filepath.display(),
                m.line_num,
                m.content
            );
            // ListItem に変換
            ListItem::new(Line::from(line_text))
        })
        .collect();

    // 検索結果のリストウィジェットを作成
    let matches_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("検索結果"));

    // --- 3. ウィジェットの描画 ---
    frame.render_widget(title, main_layout[0]); // 1行目にタイトルを描画
    frame.render_widget(matches_list, main_layout[1]); // 残りの領域にリストを描画
}