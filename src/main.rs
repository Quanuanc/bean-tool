use std::fs;
use std::path::Path;
use clap::{Parser, Subcommand};
use std::collections::HashSet;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[command(about = "Remove metadata lines from all .beancount files in a directory.")]
    RemoveMeta {
        #[arg(long, help = "Specify the txn path, will process all .beancount files recursively.")]
        txn_path: String,
    },
    #[command(about = "Merge include directives into a single .beancount file.")]
    Merge {
        #[arg(long, help = "Input .beancount file path.")]
        input: String,
        #[arg(long, help = "Output file path.")]
        output: String,
    },
}

fn main() {
    let args = Args::parse();

    match args.command {
        Command::RemoveMeta { txn_path } => {
            if !Path::new(&txn_path).exists() {
                println!("txnPath does not exist!");
                return;
            }
            process_beancount_files(&txn_path);
        }
        Command::Merge { input, output } => {
            if !Path::new(&input).exists() {
                println!("input file does not exist!");
                return;
            }
            if Path::new(&input).extension().unwrap_or_default() != "beancount" {
                println!("input must be a .beancount file!");
                return;
            }
            let mut visited = HashSet::new();
            let merged = merge_beancount_file(&input, &mut visited);
            fs::write(output, merged).unwrap();
        }
    }
}

fn process_beancount_files(directory: &str) {
    let paths = fs::read_dir(directory).unwrap();

    for entry in paths {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_dir() {
            // Recursively process subdirectories
            process_beancount_files(path.to_str().unwrap());
        } else if path.is_file() && path.extension().unwrap_or_default() == "beancount" {
            process_file(path.to_str().unwrap());
        }
    }
}

fn process_file(path: &str) {
    let metas = vec![
        "category: ", "method: ", "orderId: ", "payTime: ", "source: ",
        "status: ", "type: ", "alipay_trade_no: ", "wechat_trade_no: ",
        "  note: ", "shop_trade_no: ", "timestamp: ", "trade_time: ",
        "balances: ", "currency: ", "peerAccount: ", "txType: ", "type: ",
        "trade_time: ", "merchantId: ", "peerAccountNum: ",
    ];

    let content = fs::read_to_string(path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    let mut file_modified = false;

    let filtered_lines: Vec<&str> = lines
        .into_iter()
        .filter(|line| {
            for meta in &metas {
                if line.contains(meta) {
                    file_modified = true;
                    return false;
                }
            }
            true
        })
        .collect();

    if file_modified {
        let new_content = filtered_lines.join("\n");
        fs::write(path, new_content).unwrap();
    }
}

fn merge_beancount_file(path: &str, visited: &mut HashSet<String>) -> String {
    let canonical = fs::canonicalize(path).unwrap();
    let canonical_str = canonical.to_string_lossy().to_string();
    if !visited.insert(canonical_str) {
        // Prevent include cycles from repeating content.
        return String::new();
    }

    let content = fs::read_to_string(path).unwrap();
    let mut output = String::new();
    let base_dir = Path::new(path).parent().unwrap_or_else(|| Path::new("."));

    for line in content.lines() {
        if let Some(include_path) = parse_include_path(line) {
            let include_full = base_dir.join(include_path);
            let merged = merge_beancount_file(include_full.to_str().unwrap(), visited);
            output.push_str(&merged);
            if !merged.ends_with('\n') && !merged.is_empty() {
                output.push('\n');
            }
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }

    output
}

fn parse_include_path(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("include") {
        return None;
    }
    let rest = trimmed.strip_prefix("include")?.trim_start();
    if rest.is_empty() {
        return None;
    }

    // Accept quoted includes: include "path" or include 'path'
    let first = rest.chars().next()?;
    if first == '"' || first == '\'' {
        let mut chars = rest.chars();
        chars.next();
        let mut end = 0;
        for c in chars {
            end += 1;
            if c == first {
                return Some(&rest[1..end]);
            }
        }
        return None;
    }

    // Fallback: include path (no quotes)
    Some(rest.split_whitespace().next().unwrap_or(""))
}
