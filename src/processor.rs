//! JSON 파일 처리 모듈
//!
//! 개별 JSON 파일의 읽기, 파싱, 변환을 담당합니다.

use memmap2::Mmap;
use serde_json::{Map, Value};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use crate::error::{JConvertError, Result};

/// 파일 처리 결과
#[derive(Debug)]
pub struct ProcessResult {
    /// 처리된 파일 경로
    pub path: PathBuf,
    /// 변환된 JSON 라인 (성공 시)
    pub json_line: Option<String>,
    /// 에러 메시지 (실패 시)
    pub error: Option<String>,
    /// 원본 파일 크기
    pub file_size: u64,
    /// JSON 유효성 여부
    pub is_valid: bool,
}

impl ProcessResult {
    /// 성공 결과 생성
    pub fn success(path: PathBuf, json_line: String, file_size: u64) -> Self {
        Self {
            path,
            json_line: Some(json_line),
            error: None,
            file_size,
            is_valid: true,
        }
    }

    /// 실패 결과 생성
    pub fn failure(path: PathBuf, error: String, file_size: u64) -> Self {
        Self {
            path,
            json_line: None,
            error: Some(error),
            file_size,
            is_valid: false,
        }
    }

    /// 유효성 검사 성공 결과 생성
    pub fn valid(path: PathBuf, file_size: u64) -> Self {
        Self {
            path,
            json_line: None,
            error: None,
            file_size,
            is_valid: true,
        }
    }
}

/// JSON 처리 옵션
#[derive(Debug, Clone, Default)]
pub struct ProcessOptions {
    /// 추출할 필드 목록 (None이면 전체)
    pub fields: Option<Vec<String>>,
    /// Pretty 출력 여부
    pub pretty: bool,
    /// 유효성 검사만 수행
    pub validate_only: bool,
    /// 대용량 파일 임계값 (이상이면 메모리 매핑 사용)
    pub mmap_threshold: u64,
}

impl ProcessOptions {
    /// 기본 옵션 생성
    pub fn new() -> Self {
        Self {
            mmap_threshold: 10 * 1024 * 1024, // 10MB
            ..Default::default()
        }
    }

    /// 필드 선택 옵션 설정
    pub fn with_fields(mut self, fields: Option<Vec<String>>) -> Self {
        self.fields = fields;
        self
    }

    /// Pretty 출력 설정
    pub fn with_pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }

    /// 유효성 검사 모드 설정
    pub fn with_validate_only(mut self, validate_only: bool) -> Self {
        self.validate_only = validate_only;
        self
    }
}

/// 단일 JSON 파일 처리
///
/// # Arguments
/// * `path` - 처리할 JSON 파일 경로
/// * `options` - 처리 옵션
///
/// # Returns
/// 처리 결과를 담은 `ProcessResult`
pub fn process_file(path: PathBuf, options: &ProcessOptions) -> ProcessResult {
    let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

    match process_file_internal(&path, file_size, options) {
        Ok(json_line) => {
            if options.validate_only {
                ProcessResult::valid(path, file_size)
            } else {
                ProcessResult::success(path, json_line, file_size)
            }
        }
        Err(e) => ProcessResult::failure(path, e.to_string(), file_size),
    }
}

/// 내부 파일 처리 로직
fn process_file_internal(
    path: &PathBuf,
    file_size: u64,
    options: &ProcessOptions,
) -> Result<String> {
    let json: Value = if file_size >= options.mmap_threshold {
        // 대용량 파일: 메모리 매핑 사용
        parse_with_mmap(path)?
    } else {
        // 일반 파일: 버퍼 리더 사용
        parse_with_reader(path)?
    };

    // 유효성 검사만 하는 경우
    if options.validate_only {
        return Ok(String::new());
    }

    // 필드 선택 처리
    let output_json = match &options.fields {
        Some(fields) => extract_fields(&json, fields),
        None => json,
    };

    // JSON 직렬화
    let json_line = if options.pretty {
        serde_json::to_string_pretty(&output_json)
    } else {
        serde_json::to_string(&output_json)
    }
    .map_err(|e| JConvertError::SerializeError {
        file: path.clone(),
        reason: e.to_string(),
    })?;

    Ok(json_line)
}

/// 버퍼 리더를 사용한 JSON 파싱
fn parse_with_reader(path: &PathBuf) -> Result<Value> {
    let file = File::open(path).map_err(|e| JConvertError::FileOpenError {
        file: path.clone(),
        reason: e.to_string(),
    })?;

    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|e| JConvertError::ParseError {
        file: path.clone(),
        reason: e.to_string(),
    })
}

/// 메모리 매핑을 사용한 JSON 파싱 (대용량 파일용)
fn parse_with_mmap(path: &PathBuf) -> Result<Value> {
    let file = File::open(path).map_err(|e| JConvertError::FileOpenError {
        file: path.clone(),
        reason: e.to_string(),
    })?;

    let mmap = unsafe {
        Mmap::map(&file).map_err(|e| JConvertError::FileOpenError {
            file: path.clone(),
            reason: format!("메모리 매핑 실패: {}", e),
        })?
    };

    serde_json::from_slice(&mmap).map_err(|e| JConvertError::ParseError {
        file: path.clone(),
        reason: e.to_string(),
    })
}

/// JSON에서 특정 필드만 추출
///
/// # Arguments
/// * `json` - 원본 JSON 값
/// * `fields` - 추출할 필드 이름 목록
///
/// # Returns
/// 선택된 필드만 포함된 새 JSON 객체
fn extract_fields(json: &Value, fields: &[String]) -> Value {
    match json {
        Value::Object(map) => {
            let mut new_map = Map::new();
            for field in fields {
                // 중첩 필드 지원 (예: "user.name")
                if field.contains('.') {
                    if let Some(value) = get_nested_field(json, field) {
                        // 중첩 필드를 평탄화하여 저장
                        let flat_key = field.replace('.', "_");
                        new_map.insert(flat_key, value.clone());
                    }
                } else if let Some(value) = map.get(field) {
                    new_map.insert(field.clone(), value.clone());
                }
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => {
            // 배열인 경우 각 요소에 필드 추출 적용
            Value::Array(
                arr.iter()
                    .map(|item| extract_fields(item, fields))
                    .collect(),
            )
        }
        _ => json.clone(),
    }
}

/// 중첩 필드 값 가져오기 (예: "user.profile.name")
fn get_nested_field<'a>(json: &'a Value, path: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = json;

    for part in parts {
        match current {
            Value::Object(map) => {
                current = map.get(part)?;
            }
            Value::Array(arr) => {
                // 숫자 인덱스 처리
                if let Ok(index) = part.parse::<usize>() {
                    current = arr.get(index)?;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }

    Some(current)
}

/// JSON 파일 유효성 검사만 수행
///
/// # Arguments
/// * `path` - 검사할 JSON 파일 경로
///
/// # Returns
/// 유효성 검사 결과
pub fn validate_file(path: PathBuf) -> ProcessResult {
    let options = ProcessOptions::new().with_validate_only(true);
    process_file(path, &options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_fields_simple() {
        let json = json!({
            "id": 1,
            "name": "test",
            "description": "A test item",
            "extra": "not needed"
        });

        let fields = vec!["id".to_string(), "name".to_string()];
        let result = extract_fields(&json, &fields);

        assert_eq!(result.get("id"), Some(&json!(1)));
        assert_eq!(result.get("name"), Some(&json!("test")));
        assert_eq!(result.get("description"), None);
        assert_eq!(result.get("extra"), None);
    }

    #[test]
    fn test_extract_fields_nested() {
        let json = json!({
            "user": {
                "name": "John",
                "profile": {
                    "age": 30
                }
            }
        });

        let fields = vec!["user.name".to_string(), "user.profile.age".to_string()];
        let result = extract_fields(&json, &fields);

        assert_eq!(result.get("user_name"), Some(&json!("John")));
        assert_eq!(result.get("user_profile_age"), Some(&json!(30)));
    }

    #[test]
    fn test_extract_fields_array() {
        let json = json!([
            {"id": 1, "name": "a", "extra": "x"},
            {"id": 2, "name": "b", "extra": "y"}
        ]);

        let fields = vec!["id".to_string(), "name".to_string()];
        let result = extract_fields(&json, &fields);

        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].get("id"), Some(&json!(1)));
        assert_eq!(arr[0].get("extra"), None);
    }

    #[test]
    fn test_get_nested_field() {
        let json = json!({
            "a": {
                "b": {
                    "c": "value"
                }
            }
        });

        assert_eq!(get_nested_field(&json, "a.b.c"), Some(&json!("value")));
        assert_eq!(get_nested_field(&json, "a.b"), Some(&json!({"c": "value"})));
        assert_eq!(get_nested_field(&json, "a.x"), None);
    }

    #[test]
    fn test_process_options_builder() {
        let options = ProcessOptions::new()
            .with_fields(Some(vec!["id".to_string()]))
            .with_pretty(true)
            .with_validate_only(false);

        assert_eq!(options.fields, Some(vec!["id".to_string()]));
        assert!(options.pretty);
        assert!(!options.validate_only);
    }
}
