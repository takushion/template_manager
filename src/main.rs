
use std::fs::File; // ファイルを扱うためのモジュール
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use clap::Parser;
//use walkdir::WalkDir;
use ignore::Walk;

#[derive(Debug)] // デバッグ表示できるようにする
struct Match {
    filepath: PathBuf,
    line_num: usize,
    content: String,
}

#[derive(Parser,Debug)]
#[command(version, about = "指定されたファイルからキーワードを検索します")]

struct Cli {
    /// 検索するキーワード
    #[arg(required = true)]
    keyword: String,

    /// 検索対象のファイルパス
    #[arg(required = true)]
    target_dir: PathBuf,

    #[arg(short = 'i', long = "ignore-case")]
    ignore_case: bool,
}

fn main() {
    let cli = Cli::parse();

    let mut all_matches: Vec<Match> =Vec::new();

    for result in Walk::new(&cli.target_dir) {
        match result {
            Ok(entry) => {
                // entry.file_type() は Option<FileType> を返す
                // ignore クレートが自動で除外設定を読み込む
                if let Some(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let filepath = entry.path().to_path_buf();

                        // (重要) 1ファイルごとに find_matches を呼び出す
                        match find_matches(&filepath, &cli.keyword, cli.ignore_case) {
                            Ok(matches) => {
                                all_matches.extend(matches);
                            }
                            Err(e) => {
                                // バイナリファイルはignoreが除外してくれるはずだが、
                                // 念のためUTF-8以外のエラーも考慮
                                eprintln!("警告: ファイル {} の処理中にエラー: {}", filepath.display(), e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("警告: ファイル探索中にエラーが発生しました: {}", e);
            }
        }
    }

    if all_matches.is_empty() {
        println!("キーワード '{}' に一致する結果は見つかりませんでした。", cli.keyword);
    } else {
        for m in all_matches {
            println!("{}:{}: {}", m.filepath.display(), m.line_num, m.content);
        }
    }
}

fn find_matches(filepath: &PathBuf, keyword: &str, ignore_case: bool) -> Result<Vec<Match>, Box<dyn std::error::Error>> {


    let file = File::open(filepath)?; // ?演算子でエラーを自動的に呼び出し元に返す
    let reader = BufReader::new(file);

    let mut results: Vec<Match> = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        // .map_err() でエラーの種類を変換
        let line_content = line.map_err(|e| format!("行の読み込みに失敗しました: {}", e))?;

        let mut is_found =false;

        if ignore_case {
            if line_content.to_lowercase().contains(&keyword.to_lowercase()) {
                is_found = true;
            }

        } else {
            if line_content.contains(keyword) {
                is_found = true;
            }
        }


        if is_found {
            // ...println! する代わりに、Match構造体を作成して...
            let m = Match {
                filepath: filepath.clone(), // PathBufをコピー
                line_num: index + 1,        // 行番号
                content: line_content.trim().to_string(), // 内容
            };
            // ...ベクターに追加 (push) する
            results.push(m);
        }
    }

    Ok(results)

}