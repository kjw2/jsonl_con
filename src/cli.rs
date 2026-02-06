//! CLI 인자 파싱 모듈
//!
//! clap을 사용한 명령줄 인자 정의 및 파싱을 담당합니다.

use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// 출력 파일 모드
#[derive(Debug, Clone, Copy, ValueEnum, Default, PartialEq)]
pub enum WriteMode {
    /// 기존 파일이 있으면 덮어쓰기
    #[default]
    Overwrite,
    /// 기존 파일에 추가
    Append,
    /// 기존 파일이 있으면 에러
    Error,
}

impl std::fmt::Display for WriteMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteMode::Overwrite => write!(f, "Overwrite"),
            WriteMode::Append => write!(f, "Append"),
            WriteMode::Error => write!(f, "Error"),
        }
    }
}

/// jconvert CLI 인자 구조체
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
  • 필드 선택 기능으로 필요한 데이터만 추출
  • 상세한 오류 보고

예제:
  jconvert -i ./data -o result.jsonl
  jconvert -i ./data -o result.jsonl --mode append
  jconvert -i ./data -o result.jsonl --verbose --dry-run
  jconvert -i ./data --validate-only
  jconvert -i ./data --fields "id,name,description"
"#
)]
pub struct Args {
    /// JSON 파일들이 있는 입력 폴더 경로
    #[arg(short, long)]
    pub input: PathBuf,

    /// 생성될 JSONL 파일 경로 (기본값: output.jsonl)
    #[arg(short, long, default_value = "output.jsonl")]
    pub output: PathBuf,

    /// 출력 파일 모드
    #[arg(short, long, value_enum, default_value_t = WriteMode::Overwrite)]
    pub mode: WriteMode,

    /// 파일 이름 패턴 필터 (glob 형식, 예: "*_SUM_*", "data?.json")
    #[arg(short, long)]
    pub pattern: Option<String>,

    /// 상세 출력 모드
    #[arg(short, long)]
    pub verbose: bool,

    /// 실제 병합 없이 처리될 파일 목록만 표시
    #[arg(long)]
    pub dry_run: bool,

    /// JSON 유효성 검사만 수행 (변환 없음)
    #[arg(long)]
    pub validate_only: bool,

    /// 추출할 JSON 필드 (쉼표로 구분, 예: "id,name,title")
    #[arg(long)]
    pub fields: Option<String>,

    /// 병렬 처리 스레드 수 (기본값: CPU 코어 수)
    #[arg(short = 'j', long)]
    pub threads: Option<usize>,

    /// 최대 폴더 탐색 깊이
    #[arg(long)]
    pub max_depth: Option<usize>,

    /// 에러 로그 파일 경로
    #[arg(long)]
    pub log: Option<PathBuf>,

    /// 압축된 JSON 출력 (기본값: 압축)
    #[arg(long)]
    pub pretty: bool,
}

impl Args {
    /// 필드 목록을 파싱하여 벡터로 반환
    pub fn get_fields(&self) -> Option<Vec<String>> {
        self.fields.as_ref().map(|f| {
            f.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
    }
}
