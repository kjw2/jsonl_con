use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use serde_json::Value;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use walkdir::WalkDir;

/// 출력 파일 모드
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum WriteMode {
    /// 기존 파일이 있으면 덮어쓰기
    #[default]
    Overwrite,
    /// 기존 파일에 추가
    Append,
    /// 기존 파일이 있으면 에러
    Error,
}

#[derive(Parser, Debug)]
#[command(
    name = "jconvert",
    author = "YourName <your@email.com>",
    version,
    about = "JSON FOLDER TO JSONL CONVERTER - 폴더 내 JSON 파일들을 JSONL로 병합하는 고성능 CLI 도구",
    long_about = r#"
JSON FOLDER TO JSONL CONVERTER
==============================

지정된 폴더 내의 모든 JSON 파일을 탐색하여 
하나의 JSONL (JSON Lines) 파일로 병합합니다.

특징:
  • 병렬 처리로 대량 파일 고속 변환
  • 진행률 표시 및 상세 통계
  • 다양한 출력 모드 지원 (덮어쓰기/추가/에러)
  • 상세한 오류 보고

예제:
  jconvert -i ./data -o result.jsonl
  jconvert -i ./data -o result.jsonl --mode append
  jconvert -i ./data -o result.jsonl --verbose --dry-run
"#
)]
struct Args {
    /// JSON 파일들이 있는 입력 폴더 경로
    #[arg(short, long)]
    input: PathBuf,

    /// 생성될 JSONL 파일 경로 (기본값: output.jsonl)
    #[arg(short, long, default_value = "output.jsonl")]
    output: PathBuf,

    /// 출력 파일 모드
    #[arg(short, long, value_enum, default_value_t = WriteMode::Overwrite)]
    mode: WriteMode,

    /// 파일 이름 패턴 필터 (예: "*_SUM_*")
    #[arg(short, long)]
    pattern: Option<String>,

    /// 상세 출력 모드
    #[arg(short, long)]
    verbose: bool,

    /// 실제 병합 없이 처리될 파일 목록만 표시
    #[arg(long)]
    dry_run: bool,

    /// 병렬 처리 스레드 수 (기본값: CPU 코어 수)
    #[arg(short = 'j', long)]
    threads: Option<usize>,
}

/// 파일 처리 결과
#[derive(Debug)]
struct ProcessResult {
    path: PathBuf,
    json_line: Option<String>,
    error: Option<String>,
    file_size: u64,
}

/// 처리 통계
#[derive(Debug, Default)]
struct Statistics {
    total_files: usize,
    success_count: AtomicUsize,
    error_count: AtomicUsize,
    total_bytes_read: AtomicU64,
    total_bytes_written: AtomicU64,
}

impl Statistics {
    fn print_summary(&self) {
        println!("\n{}", "═".repeat(50).bright_blue());
        println!("{}", " 📊 처리 통계".bright_white().bold());
        println!("{}", "═".repeat(50).bright_blue());

        let success = self.success_count.load(Ordering::Relaxed);
        let errors = self.error_count.load(Ordering::Relaxed);
        let bytes_read = self.total_bytes_read.load(Ordering::Relaxed);
        let bytes_written = self.total_bytes_written.load(Ordering::Relaxed);

        println!(
            "  {} 전체 파일:    {}",
            "📁".bright_cyan(),
            self.total_files
        );
        println!(
            "  {} 성공:         {}",
            "✅".bright_green(),
            success.to_string().green()
        );

        if errors > 0 {
            println!(
                "  {} 실패:         {}",
                "❌".bright_red(),
                errors.to_string().red()
            );
        } else {
            println!("  {} 실패:         {}", "✅".bright_green(), "0".green());
        }

        println!(
            "  {} 입력 용량:    {}",
            "📥".bright_yellow(),
            format_bytes(bytes_read)
        );
        println!(
            "  {} 출력 용량:    {}",
            "📤".bright_magenta(),
            format_bytes(bytes_written)
        );

        if self.total_files > 0 {
            let success_rate = (success as f64 / self.total_files as f64) * 100.0;
            println!(
                "  {} 성공률:       {:.1}%",
                "📈".bright_white(),
                success_rate
            );
        }

        println!("{}", "═".repeat(50).bright_blue());
    }
}

/// 바이트를 읽기 쉬운 형식으로 변환
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// 파일 이름이 패턴과 일치하는지 확인
fn matches_pattern(file_name: &str, pattern: &Option<String>) -> bool {
    match pattern {
        None => true,
        Some(pat) => {
            // 간단한 와일드카드 패턴 매칭 (* 지원)
            let parts: Vec<&str> = pat.split('*').collect();
            if parts.len() == 1 {
                file_name.contains(pat)
            } else {
                let mut pos = 0;
                for (i, part) in parts.iter().enumerate() {
                    if part.is_empty() {
                        continue;
                    }
                    if let Some(found) = file_name[pos..].find(part) {
                        if i == 0 && found != 0 {
                            return false; // 패턴이 *로 시작하지 않으면 처음부터 매칭
                        }
                        pos += found + part.len();
                    } else {
                        return false;
                    }
                }
                true
            }
        }
    }
}

/// 단일 JSON 파일 처리
fn process_file(path: PathBuf) -> ProcessResult {
    let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

    let result = (|| -> Result<String> {
        let file = File::open(&path).with_context(|| format!("파일 열기 실패: {:?}", path))?;

        let reader = std::io::BufReader::new(file);
        let json: Value = serde_json::from_reader(reader)
            .with_context(|| format!("JSON 파싱 실패: {:?}", path))?;

        serde_json::to_string(&json).with_context(|| format!("JSON 직렬화 실패: {:?}", path))
    })();

    match result {
        Ok(json_line) => ProcessResult {
            path,
            json_line: Some(json_line),
            error: None,
            file_size,
        },
        Err(e) => ProcessResult {
            path,
            json_line: None,
            error: Some(e.to_string()),
            file_size,
        },
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    // 스레드 풀 설정
    if let Some(threads) = args.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .context("스레드 풀 초기화 실패")?;
    }

    // 입력 폴더 확인
    if !args.input.exists() {
        anyhow::bail!("입력 폴더가 존재하지 않습니다: {:?}", args.input);
    }

    if !args.input.is_dir() {
        anyhow::bail!("입력 경로가 폴더가 아닙니다: {:?}", args.input);
    }

    println!("\n{}", "═".repeat(50).bright_blue());
    println!(
        "{}",
        " 🚀 JSON FOLDER TO JSONL CONVERTER".bright_white().bold()
    );
    println!("{}", "═".repeat(50).bright_blue());
    println!("  {} 입력 폴더: {:?}", "📂".bright_cyan(), args.input);
    println!("  {} 출력 파일: {:?}", "📄".bright_green(), args.output);
    println!("  {} 모드: {:?}", "⚙️".bright_yellow(), args.mode);

    if let Some(ref pattern) = args.pattern {
        println!("  {} 패턴 필터: {}", "🔍".bright_magenta(), pattern);
    }

    if args.dry_run {
        println!(
            "  {} {}",
            "⚠️".bright_yellow(),
            "드라이런 모드 (실제 병합 없음)".yellow()
        );
    }

    println!("{}", "═".repeat(50).bright_blue());

    // JSON 파일 수집
    println!("\n{}", "📁 파일 검색 중...".bright_cyan());

    let json_files: Vec<PathBuf> = WalkDir::new(&args.input)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("json"))
                .unwrap_or(false)
        })
        .filter(|e| {
            e.path()
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| matches_pattern(s, &args.pattern))
                .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    if json_files.is_empty() {
        println!("{}", "⚠️ 처리할 JSON 파일이 없습니다.".yellow());
        return Ok(());
    }

    println!(
        "  {} 발견된 파일 수: {}",
        "📋".bright_white(),
        json_files.len().to_string().bright_green()
    );

    // 통계 초기화
    let stats = Statistics {
        total_files: json_files.len(),
        ..Default::default()
    };

    // 드라이런 모드
    if args.dry_run {
        println!("\n{}", "📋 처리 예정 파일 목록:".bright_cyan());
        for (i, path) in json_files.iter().enumerate() {
            println!("  {}. {:?}", i + 1, path.file_name().unwrap_or_default());
        }
        println!(
            "\n{} 총 {} 개의 파일이 처리될 예정입니다.",
            "ℹ️".bright_blue(),
            json_files.len().to_string().bright_green()
        );
        return Ok(());
    }

    // 출력 파일 모드 확인
    match args.mode {
        WriteMode::Error if args.output.exists() => {
            anyhow::bail!("출력 파일이 이미 존재합니다: {:?}", args.output);
        }
        _ => {}
    }

    // 진행률 바 설정
    let pb = ProgressBar::new(json_files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")?
            .progress_chars("█▓▒░"),
    );

    // 병렬 처리
    println!("\n{}", "⚡ 병렬 처리 중...".bright_cyan());

    let results: Vec<ProcessResult> = json_files
        .into_par_iter()
        .map(|path| {
            let result = process_file(path);
            pb.inc(1);
            result
        })
        .collect();

    pb.finish_with_message("완료!");

    // 결과 수집 및 파일 쓰기
    println!("\n{}", "💾 JSONL 파일 저장 중...".bright_cyan());

    let output_file = match args.mode {
        WriteMode::Append => OpenOptions::new()
            .create(true)
            .append(true)
            .open(&args.output)?,
        _ => File::create(&args.output)?,
    };

    let writer = Mutex::new(BufWriter::new(output_file));
    let mut errors: Vec<(PathBuf, String)> = Vec::new();

    for result in results {
        if let Some(json_line) = result.json_line {
            let line_bytes = json_line.len() as u64 + 1; // +1 for newline
            stats
                .total_bytes_read
                .fetch_add(result.file_size, Ordering::Relaxed);
            stats
                .total_bytes_written
                .fetch_add(line_bytes, Ordering::Relaxed);
            stats.success_count.fetch_add(1, Ordering::Relaxed);

            let mut w = writer.lock().unwrap();
            writeln!(w, "{}", json_line)?;

            if args.verbose {
                println!(
                    "  {} {:?}",
                    "✓".green(),
                    result.path.file_name().unwrap_or_default()
                );
            }
        } else if let Some(error) = result.error {
            stats.error_count.fetch_add(1, Ordering::Relaxed);
            errors.push((result.path, error));
        }
    }

    // 버퍼 플러시
    writer.lock().unwrap().flush()?;

    // 오류 목록 출력
    if !errors.is_empty() {
        println!("\n{}", "❌ 오류 발생 파일:".bright_red());
        for (path, error) in &errors {
            println!("  {} {:?}", "•".red(), path.file_name().unwrap_or_default());
            if args.verbose {
                println!("    {}", error.dimmed());
            }
        }
    }

    // 통계 출력
    stats.print_summary();

    println!("\n{} 저장 완료: {:?}\n", "✅".bright_green(), args.output);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_pattern() {
        assert!(matches_pattern(
            "test_SUM_1.json",
            &Some("*_SUM_*".to_string())
        ));
        assert!(matches_pattern(
            "HS_H_323503_SUM_15.json",
            &Some("*_SUM_*".to_string())
        ));
        assert!(!matches_pattern("test.json", &Some("*_SUM_*".to_string())));
        assert!(matches_pattern("anything.json", &None));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }
}
