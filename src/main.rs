//! jconvert - JSON FOLDER TO JSONL CONVERTER
//!
//! 메인 엔트리포인트

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use walkdir::WalkDir;

use jconvert::{
    cli::{Args, WriteMode},
    pattern::PatternMatcher,
    processor::{process_file, ProcessOptions, ProcessResult},
    stats::Statistics,
};

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
    validate_input(&args)?;

    // 헤더 출력
    print_header(&args);

    // 패턴 매처 초기화
    let pattern_matcher =
        PatternMatcher::new(args.pattern.clone()).map_err(|e| anyhow::anyhow!("{}", e))?;

    // JSON 파일 수집
    let json_files = collect_json_files(&args, &pattern_matcher)?;

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
    let stats = Statistics::new(json_files.len());

    // 드라이런 모드
    if args.dry_run {
        print_dry_run(&json_files);
        return Ok(());
    }

    // 유효성 검사 모드
    if args.validate_only {
        return run_validation_mode(&args, json_files, &stats);
    }

    // 일반 변환 모드
    run_conversion_mode(&args, json_files, &stats)
}

/// 입력 경로 유효성 검사
fn validate_input(args: &Args) -> Result<()> {
    if !args.input.exists() {
        anyhow::bail!("입력 폴더가 존재하지 않습니다: {:?}", args.input);
    }

    if !args.input.is_dir() {
        anyhow::bail!("입력 경로가 폴더가 아닙니다: {:?}", args.input);
    }

    Ok(())
}

/// 헤더 출력
fn print_header(args: &Args) {
    println!("\n{}", "═".repeat(50).bright_blue());
    println!(
        "{}",
        " 🚀 JSON FOLDER TO JSONL CONVERTER".bright_white().bold()
    );
    println!("{}", "═".repeat(50).bright_blue());
    println!("  {} 입력 폴더: {:?}", "📂".bright_cyan(), args.input);

    if !args.validate_only {
        println!("  {} 출력 파일: {:?}", "📄".bright_green(), args.output);
        println!("  {} 모드: {}", "⚙️".bright_yellow(), args.mode);
    }

    if let Some(ref pattern) = args.pattern {
        println!("  {} 패턴 필터: {}", "🔍".bright_magenta(), pattern);
    }

    if let Some(ref fields) = args.fields {
        println!("  {} 필드 선택: {}", "🎯".bright_cyan(), fields);
    }

    if let Some(depth) = args.max_depth {
        println!("  {} 최대 깊이: {}", "📏".bright_white(), depth);
    }

    if args.dry_run {
        println!(
            "  {} {}",
            "⚠️".bright_yellow(),
            "드라이런 모드 (실제 병합 없음)".yellow()
        );
    }

    if args.validate_only {
        println!("  {} {}", "🔍".bright_cyan(), "유효성 검사 모드".cyan());
    }

    if args.pretty {
        println!(
            "  {} {}",
            "✨".bright_magenta(),
            "Pretty 출력 모드".magenta()
        );
    }

    println!("{}", "═".repeat(50).bright_blue());
    println!("\n{}", "📁 파일 검색 중...".bright_cyan());
}

/// JSON 파일 수집
fn collect_json_files(args: &Args, pattern_matcher: &PatternMatcher) -> Result<Vec<PathBuf>> {
    let walker = if let Some(max_depth) = args.max_depth {
        WalkDir::new(&args.input).max_depth(max_depth)
    } else {
        WalkDir::new(&args.input)
    };

    let json_files: Vec<PathBuf> = walker
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
                .map(|s| pattern_matcher.matches(s))
                .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    Ok(json_files)
}

/// 드라이런 출력
fn print_dry_run(json_files: &[PathBuf]) {
    println!("\n{}", "📋 처리 예정 파일 목록:".bright_cyan());
    for (i, path) in json_files.iter().enumerate() {
        println!("  {}. {:?}", i + 1, path.file_name().unwrap_or_default());
    }
    println!(
        "\n{} 총 {} 개의 파일이 처리될 예정입니다.",
        "ℹ️".bright_blue(),
        json_files.len().to_string().bright_green()
    );
}

/// 유효성 검사 모드 실행
fn run_validation_mode(args: &Args, json_files: Vec<PathBuf>, stats: &Statistics) -> Result<()> {
    // 진행률 바 설정
    let pb = create_progress_bar(json_files.len());

    println!("\n{}", "🔍 유효성 검사 중...".bright_cyan());

    let options = ProcessOptions::new().with_validate_only(true);
    let errors: Mutex<Vec<(PathBuf, String)>> = Mutex::new(Vec::new());

    json_files.into_par_iter().for_each(|path| {
        let result = process_file(path, &options);
        pb.inc(1);

        if result.is_valid {
            stats.increment_success();
            stats.add_bytes_read(result.file_size);

            if args.verbose {
                println!(
                    "  {} {:?}",
                    "✓".green(),
                    result.path.file_name().unwrap_or_default()
                );
            }
        } else {
            stats.increment_validation_failed();
            if let Some(error) = result.error {
                errors.lock().unwrap().push((result.path, error));
            }
        }
    });

    pb.finish_with_message("완료!");

    // 에러 출력
    let errors = errors.into_inner().unwrap();
    print_errors(&errors, args.verbose);

    // 로그 파일 작성
    if let Some(ref log_path) = args.log {
        write_error_log(log_path, &errors)?;
    }

    // 통계 출력
    stats.print_validation_summary();

    if stats.get_validation_failed() == 0 {
        println!("\n{} 모든 파일이 유효합니다!\n", "✅".bright_green());
    } else {
        println!(
            "\n{} {} 개의 파일에 오류가 있습니다.\n",
            "⚠️".bright_yellow(),
            stats.get_validation_failed().to_string().red()
        );
    }

    Ok(())
}

/// 변환 모드 실행
fn run_conversion_mode(args: &Args, json_files: Vec<PathBuf>, stats: &Statistics) -> Result<()> {
    // 출력 파일 모드 확인
    check_output_mode(args)?;

    // 진행률 바 설정
    let pb = create_progress_bar(json_files.len());

    // 처리 옵션 생성
    let options = ProcessOptions::new()
        .with_fields(args.get_fields())
        .with_pretty(args.pretty);

    // 병렬 처리
    println!("\n{}", "⚡ 병렬 처리 중...".bright_cyan());

    let results: Vec<ProcessResult> = json_files
        .into_par_iter()
        .map(|path| {
            let result = process_file(path, &options);
            pb.inc(1);
            result
        })
        .collect();

    pb.finish_with_message("완료!");

    // 결과 수집 및 파일 쓰기
    println!("\n{}", "💾 JSONL 파일 저장 중...".bright_cyan());

    let output_file = open_output_file(args)?;
    let writer = Mutex::new(BufWriter::new(output_file));
    let mut errors: Vec<(PathBuf, String)> = Vec::new();

    for result in results {
        if let Some(json_line) = result.json_line {
            let line_bytes = json_line.len() as u64 + 1; // +1 for newline
            stats.add_bytes_read(result.file_size);
            stats.add_bytes_written(line_bytes);
            stats.increment_success();

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
            stats.increment_error();
            errors.push((result.path, error));
        }
    }

    // 버퍼 플러시
    writer.lock().unwrap().flush()?;

    // 에러 출력
    print_errors(&errors, args.verbose);

    // 로그 파일 작성
    if let Some(ref log_path) = args.log {
        write_error_log(log_path, &errors)?;
    }

    // 통계 출력
    stats.print_summary();

    println!("\n{} 저장 완료: {:?}\n", "✅".bright_green(), args.output);

    Ok(())
}

/// 출력 모드 확인
fn check_output_mode(args: &Args) -> Result<()> {
    if args.mode == WriteMode::Error && args.output.exists() {
        anyhow::bail!("출력 파일이 이미 존재합니다: {:?}", args.output);
    }
    Ok(())
}

/// 출력 파일 열기
fn open_output_file(args: &Args) -> Result<File> {
    let file = match args.mode {
        WriteMode::Append => OpenOptions::new()
            .create(true)
            .append(true)
            .open(&args.output)?,
        _ => File::create(&args.output)?,
    };
    Ok(file)
}

/// 진행률 바 생성
fn create_progress_bar(total: usize) -> ProgressBar {
    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
            .unwrap()
            .progress_chars("█▓▒░"),
    );
    pb
}

/// 에러 목록 출력
fn print_errors(errors: &[(PathBuf, String)], verbose: bool) {
    if errors.is_empty() {
        return;
    }

    println!("\n{}", "❌ 오류 발생 파일:".bright_red());
    for (path, error) in errors {
        println!("  {} {:?}", "•".red(), path.file_name().unwrap_or_default());
        if verbose {
            println!("    {}", error.dimmed());
        }
    }
}

/// 에러 로그 파일 작성
fn write_error_log(log_path: &PathBuf, errors: &[(PathBuf, String)]) -> Result<()> {
    let mut log_file = File::create(log_path)?;

    writeln!(log_file, "jconvert 에러 로그")?;
    writeln!(log_file, "생성 시간: {}", chrono_now())?;
    writeln!(log_file, "총 에러 수: {}", errors.len())?;
    writeln!(log_file, "{}", "=".repeat(50))?;

    for (path, error) in errors {
        writeln!(log_file, "\n파일: {:?}", path)?;
        writeln!(log_file, "에러: {}", error)?;
    }

    println!("\n{} 에러 로그 저장: {:?}", "📝".bright_cyan(), log_path);

    Ok(())
}

/// 현재 시간 문자열 반환
fn chrono_now() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now();
    let duration = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("Unix timestamp: {}", duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_json(dir: &std::path::Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_collect_json_files() {
        let temp_dir = TempDir::new().unwrap();
        create_test_json(temp_dir.path(), "test1.json", r#"{"id": 1}"#);
        create_test_json(temp_dir.path(), "test2.json", r#"{"id": 2}"#);
        create_test_json(temp_dir.path(), "other.txt", "not json");

        let args = Args {
            input: temp_dir.path().to_path_buf(),
            output: PathBuf::from("output.jsonl"),
            mode: WriteMode::Overwrite,
            pattern: None,
            verbose: false,
            dry_run: false,
            validate_only: false,
            fields: None,
            threads: None,
            max_depth: None,
            log: None,
            pretty: false,
        };

        let pattern_matcher = PatternMatcher::new(None).unwrap();
        let files = collect_json_files(&args, &pattern_matcher).unwrap();

        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_collect_json_files_with_pattern() {
        let temp_dir = TempDir::new().unwrap();
        create_test_json(temp_dir.path(), "data_SUM_1.json", r#"{"id": 1}"#);
        create_test_json(temp_dir.path(), "data_SUM_2.json", r#"{"id": 2}"#);
        create_test_json(temp_dir.path(), "other.json", r#"{"id": 3}"#);

        let args = Args {
            input: temp_dir.path().to_path_buf(),
            output: PathBuf::from("output.jsonl"),
            mode: WriteMode::Overwrite,
            pattern: Some("*_SUM_*".to_string()),
            verbose: false,
            dry_run: false,
            validate_only: false,
            fields: None,
            threads: None,
            max_depth: None,
            log: None,
            pretty: false,
        };

        let pattern_matcher = PatternMatcher::new(args.pattern.clone()).unwrap();
        let files = collect_json_files(&args, &pattern_matcher).unwrap();

        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_max_depth() {
        let temp_dir = TempDir::new().unwrap();
        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir).unwrap();
        let deep_dir = sub_dir.join("deep");
        fs::create_dir(&deep_dir).unwrap();

        create_test_json(temp_dir.path(), "root.json", r#"{"level": 0}"#);
        create_test_json(&sub_dir, "level1.json", r#"{"level": 1}"#);
        create_test_json(&deep_dir, "level2.json", r#"{"level": 2}"#);

        // max_depth = 1 (root + 1 level down)
        let args = Args {
            input: temp_dir.path().to_path_buf(),
            output: PathBuf::from("output.jsonl"),
            mode: WriteMode::Overwrite,
            pattern: None,
            verbose: false,
            dry_run: false,
            validate_only: false,
            fields: None,
            threads: None,
            max_depth: Some(2),
            log: None,
            pretty: false,
        };

        let pattern_matcher = PatternMatcher::new(None).unwrap();
        let files = collect_json_files(&args, &pattern_matcher).unwrap();

        // root.json and level1.json (not level2.json because max_depth=2 means depth 0,1)
        assert_eq!(files.len(), 2);
    }
}
