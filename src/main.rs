
use std::fs::File; // ファイルを扱うためのモジュール
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use clap::Parser;
//use walkdir::WalkDir;
use ignore::Walk;

use rayon::prelude::*;

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

fn main() {
    let cli = Cli::parse();

    let mut files_to_search: Vec<PathBuf> = Vec::new();
    for result in Walk::new(&cli.target_dir) {
        match result {
            Ok(entry) => {
                if let Some(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        files_to_search.push(entry.path().to_path_buf());
                    }
                }
            }
            Err(e) => {
                eprintln!("警告: ファイル探索中にエラーが発生しました: {}", e);
            }
        }
    }

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

    if all_matches.is_empty() {
        println!("アノテーションは見つかりませんでした。");
    } else {
        for m in all_matches {
            println!("{}:{}: {}", m.filepath.display(), m.line_num, m.content);
        }
    }
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