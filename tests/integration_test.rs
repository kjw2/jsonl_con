//! 통합 테스트 모듈
//!
//! jconvert의 전체 기능을 테스트합니다.

#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// 테스트용 JSON 파일 생성 헬퍼
fn create_json_file(dir: &std::path::Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).unwrap();
    path
}

/// 테스트용 디렉토리 구조 생성
fn setup_test_directory() -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    // 유효한 JSON 파일들
    create_json_file(
        temp_dir.path(),
        "valid1.json",
        r#"{"id": 1, "name": "First"}"#,
    );
    create_json_file(
        temp_dir.path(),
        "valid2.json",
        r#"{"id": 2, "name": "Second"}"#,
    );
    create_json_file(
        temp_dir.path(),
        "data_SUM_1.json",
        r#"{"id": 3, "type": "summary", "value": 100}"#,
    );
    create_json_file(
        temp_dir.path(),
        "data_SUM_2.json",
        r#"{"id": 4, "type": "summary", "value": 200}"#,
    );

    // 중첩된 JSON
    create_json_file(
        temp_dir.path(),
        "nested.json",
        r#"{"user": {"name": "John", "profile": {"age": 30, "city": "Seoul"}}}"#,
    );

    // 배열 JSON
    create_json_file(
        temp_dir.path(),
        "array.json",
        r#"[{"id": 1}, {"id": 2}, {"id": 3}]"#,
    );

    temp_dir
}

/// 잘못된 JSON이 포함된 테스트 디렉토리 생성
fn setup_mixed_directory() -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    create_json_file(temp_dir.path(), "valid.json", r#"{"id": 1}"#);
    create_json_file(temp_dir.path(), "invalid.json", r#"{"id": 1, broken"#);
    create_json_file(temp_dir.path(), "empty.json", r#""#);

    temp_dir
}

/// 하위 디렉토리 구조 생성
fn setup_nested_directory() -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    // 루트 레벨
    create_json_file(temp_dir.path(), "root.json", r#"{"level": 0}"#);

    // 1단계 하위
    let level1 = temp_dir.path().join("level1");
    fs::create_dir(&level1).unwrap();
    create_json_file(&level1, "file1.json", r#"{"level": 1}"#);

    // 2단계 하위
    let level2 = level1.join("level2");
    fs::create_dir(&level2).unwrap();
    create_json_file(&level2, "file2.json", r#"{"level": 2}"#);

    // 3단계 하위
    let level3 = level2.join("level3");
    fs::create_dir(&level3).unwrap();
    create_json_file(&level3, "file3.json", r#"{"level": 3}"#);

    temp_dir
}

mod pattern_tests {
    use jconvert::PatternMatcher;

    #[test]
    fn test_glob_star() {
        let matcher = PatternMatcher::new(Some("*.json".to_string())).unwrap();
        assert!(matcher.matches("test.json"));
        assert!(matcher.matches("data.json"));
        assert!(!matcher.matches("test.txt"));
    }

    #[test]
    fn test_glob_question() {
        let matcher = PatternMatcher::new(Some("file?.json".to_string())).unwrap();
        assert!(matcher.matches("file1.json"));
        assert!(matcher.matches("fileA.json"));
        assert!(!matcher.matches("file.json"));
        assert!(!matcher.matches("file12.json"));
    }

    #[test]
    fn test_glob_brackets() {
        let matcher = PatternMatcher::new(Some("[abc]*.json".to_string())).unwrap();
        assert!(matcher.matches("alpha.json"));
        assert!(matcher.matches("beta.json"));
        assert!(matcher.matches("charlie.json"));
        assert!(!matcher.matches("delta.json"));
    }

    #[test]
    fn test_complex_pattern() {
        let matcher = PatternMatcher::new(Some("data_*_[0-9].json".to_string())).unwrap();
        assert!(matcher.matches("data_test_1.json"));
        assert!(matcher.matches("data_SUM_5.json"));
        assert!(!matcher.matches("data_test_10.json")); // 10은 두 자리
        assert!(!matcher.matches("other_test_1.json"));
    }
}

mod processor_tests {
    use super::*;
    use jconvert::processor::{process_file, ProcessOptions};

    #[test]
    fn test_process_valid_json() {
        let temp_dir = TempDir::new().unwrap();
        let path = create_json_file(temp_dir.path(), "test.json", r#"{"key": "value"}"#);

        let options = ProcessOptions::new();
        let result = process_file(path, &options);

        assert!(result.is_valid);
        assert!(result.json_line.is_some());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_process_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let path = create_json_file(temp_dir.path(), "invalid.json", r#"{"broken json"#);

        let options = ProcessOptions::new();
        let result = process_file(path, &options);

        assert!(!result.is_valid);
        assert!(result.json_line.is_none());
        assert!(result.error.is_some());
    }

    #[test]
    fn test_field_selection() {
        let temp_dir = TempDir::new().unwrap();
        let path = create_json_file(
            temp_dir.path(),
            "test.json",
            r#"{"id": 1, "name": "test", "extra": "data"}"#,
        );

        let options =
            ProcessOptions::new().with_fields(Some(vec!["id".to_string(), "name".to_string()]));
        let result = process_file(path, &options);

        assert!(result.is_valid);
        let json_line = result.json_line.unwrap();
        assert!(json_line.contains("\"id\":1") || json_line.contains("\"id\": 1"));
        assert!(json_line.contains("\"name\""));
        assert!(!json_line.contains("\"extra\""));
    }

    #[test]
    fn test_nested_field_selection() {
        let temp_dir = TempDir::new().unwrap();
        let path = create_json_file(
            temp_dir.path(),
            "test.json",
            r#"{"user": {"name": "John", "age": 30}, "meta": "ignored"}"#,
        );

        let options = ProcessOptions::new().with_fields(Some(vec!["user.name".to_string()]));
        let result = process_file(path, &options);

        assert!(result.is_valid);
        let json_line = result.json_line.unwrap();
        assert!(json_line.contains("John"));
        assert!(!json_line.contains("meta"));
    }

    #[test]
    fn test_pretty_output() {
        let temp_dir = TempDir::new().unwrap();
        let path = create_json_file(temp_dir.path(), "test.json", r#"{"key": "value"}"#);

        let options = ProcessOptions::new().with_pretty(true);
        let result = process_file(path, &options);

        assert!(result.is_valid);
        let json_line = result.json_line.unwrap();
        // Pretty output should have newlines
        assert!(json_line.contains('\n'));
    }

    #[test]
    fn test_validate_only() {
        let temp_dir = TempDir::new().unwrap();
        let path = create_json_file(temp_dir.path(), "test.json", r#"{"key": "value"}"#);

        let options = ProcessOptions::new().with_validate_only(true);
        let result = process_file(path, &options);

        assert!(result.is_valid);
        // validate_only should return empty json_line
        assert!(result.json_line.is_none() || result.json_line.as_ref().unwrap().is_empty());
    }
}

mod stats_tests {
    use jconvert::stats::{format_bytes, Statistics};

    #[test]
    fn test_statistics_tracking() {
        let stats = Statistics::new(10);

        stats.increment_success();
        stats.increment_success();
        stats.increment_error();
        stats.increment_validation_failed();
        stats.add_bytes_read(1024);
        stats.add_bytes_written(512);

        assert_eq!(stats.get_success_count(), 2);
        assert_eq!(stats.get_error_count(), 1);
        assert_eq!(stats.get_validation_failed(), 1);
    }

    #[test]
    fn test_format_bytes_boundaries() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024 - 1), "1024.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }
}

mod error_tests {
    use jconvert::error::JConvertError;
    use std::path::PathBuf;

    #[test]
    fn test_error_display() {
        let error = JConvertError::InputNotFound {
            path: PathBuf::from("/nonexistent"),
        };
        let msg = error.to_string();
        assert!(msg.contains("입력 폴더를 찾을 수 없습니다"));
    }

    #[test]
    fn test_parse_error_display() {
        let error = JConvertError::ParseError {
            file: PathBuf::from("test.json"),
            reason: "unexpected token".to_string(),
        };
        let msg = error.to_string();
        assert!(msg.contains("JSON 파싱 실패"));
        assert!(msg.contains("test.json"));
    }
}

mod cli_tests {
    use jconvert::cli::Args;

    #[test]
    fn test_get_fields_parsing() {
        let args = Args {
            input: std::path::PathBuf::from("."),
            output: std::path::PathBuf::from("out.jsonl"),
            mode: jconvert::WriteMode::Overwrite,
            pattern: None,
            verbose: false,
            dry_run: false,
            validate_only: false,
            fields: Some("id, name, description".to_string()),
            threads: None,
            max_depth: None,
            log: None,
            pretty: false,
        };

        let fields = args.get_fields().unwrap();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0], "id");
        assert_eq!(fields[1], "name");
        assert_eq!(fields[2], "description");
    }

    #[test]
    fn test_get_fields_none() {
        let args = Args {
            input: std::path::PathBuf::from("."),
            output: std::path::PathBuf::from("out.jsonl"),
            mode: jconvert::WriteMode::Overwrite,
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

        assert!(args.get_fields().is_none());
    }
}
