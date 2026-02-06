//! jconvert - JSON FOLDER TO JSONL CONVERTER
//!
//! 폴더 내 JSON 파일들을 하나의 JSONL (JSON Lines) 파일로 병합하는 고성능 CLI 도구입니다.
//!
//! # 주요 기능
//!
//! - 🚀 **병렬 처리**: Rayon을 활용한 멀티스레드 처리로 대량 파일 고속 변환
//! - 📊 **진행률 표시**: 처리 진행 상황을 시각적으로 확인
//! - 📈 **상세 통계**: 성공/실패 파일 수, 입출력 용량, 성공률 등 표시
//! - 🔍 **패턴 필터링**: glob 형식의 고급 파일 이름 필터링
//! - 📝 **다양한 출력 모드**: 덮어쓰기, 추가, 에러 모드 지원
//! - 🧪 **드라이런 모드**: 실제 병합 없이 처리될 파일 목록 미리 확인
//! - ✅ **유효성 검사**: JSON 파일 유효성만 검사하는 모드
//! - 🎯 **필드 선택**: 특정 필드만 추출하여 변환
//! - 🎨 **컬러 출력**: 가독성 높은 컬러 터미널 출력
//!
//! # 예제
//!
//! ```bash
//! # 기본 사용법
//! jconvert -i ./data -o result.jsonl
//!
//! # 유효성 검사만
//! jconvert -i ./data --validate-only
//!
//! # 특정 필드만 추출
//! jconvert -i ./data -o result.jsonl --fields "id,name"
//! ```

pub mod cli;
pub mod error;
pub mod pattern;
pub mod processor;
pub mod stats;

// Re-exports for convenient access
pub use cli::{Args, WriteMode};
pub use error::{JConvertError, Result};
pub use pattern::PatternMatcher;
pub use processor::{process_file, validate_file, ProcessOptions, ProcessResult};
pub use stats::{format_bytes, Statistics};
