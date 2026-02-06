//! 에러 타입 정의 모듈
//!
//! jconvert에서 발생할 수 있는 모든 에러 타입을 정의합니다.

use std::path::PathBuf;
use thiserror::Error;

/// jconvert에서 발생할 수 있는 에러 타입
#[derive(Error, Debug)]
pub enum JConvertError {
    /// 입력 폴더가 존재하지 않음
    #[error("입력 폴더를 찾을 수 없습니다: {path}")]
    InputNotFound { path: PathBuf },

    /// 입력이 폴더가 아님
    #[error("입력 경로가 폴더가 아닙니다: {path}")]
    NotADirectory { path: PathBuf },

    /// 출력 파일이 이미 존재 (Error 모드에서)
    #[error("출력 파일이 이미 존재합니다: {path}")]
    OutputExists { path: PathBuf },

    /// JSON 파일 열기 실패
    #[error("파일을 열 수 없습니다 ({file}): {reason}")]
    FileOpenError { file: PathBuf, reason: String },

    /// JSON 파싱 실패
    #[error("JSON 파싱 실패 ({file}): {reason}")]
    ParseError { file: PathBuf, reason: String },

    /// JSON 직렬화 실패
    #[error("JSON 직렬화 실패 ({file}): {reason}")]
    SerializeError { file: PathBuf, reason: String },

    /// 파일 쓰기 실패
    #[error("파일 쓰기 실패: {reason}")]
    WriteError { reason: String },

    /// 스레드 풀 초기화 실패
    #[error("스레드 풀 초기화 실패: {reason}")]
    ThreadPoolError { reason: String },

    /// 유효하지 않은 패턴
    #[error("유효하지 않은 패턴: {pattern}")]
    InvalidPattern { pattern: String },

    /// 처리할 파일 없음
    #[error("처리할 JSON 파일이 없습니다")]
    NoFilesFound,
}

/// jconvert 결과 타입 별칭
pub type Result<T> = std::result::Result<T, JConvertError>;
