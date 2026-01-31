# JSON FOLDER TO JSONL CONVERTER (jconvert)

폴더 내 JSON 파일들을 하나의 JSONL (JSON Lines) 파일로 병합하는 고성능 CLI 도구입니다.

## ✨ 주요 기능

- 🚀 **병렬 처리**: Rayon을 활용한 멀티스레드 처리로 대량 파일 고속 변환
- 📊 **진행률 표시**: 처리 진행 상황을 시각적으로 확인
- 📈 **상세 통계**: 성공/실패 파일 수, 입출력 용량, 성공률 등 표시
- 🔍 **패턴 필터링**: 와일드카드를 사용한 파일 이름 필터링
- 📝 **다양한 출력 모드**: 덮어쓰기, 추가, 에러 모드 지원
- 🧪 **드라이런 모드**: 실제 병합 없이 처리될 파일 목록 미리 확인
- 🎨 **컬러 출력**: 가독성 높은 컬러 터미널 출력

## 📦 설치

### 소스에서 빌드

```bash
# 저장소 클론
git clone <repository-url>
cd jsonl_con

# 릴리스 빌드
cargo build --release

# 실행 파일은 target/release/jconvert.exe 에 생성됩니다
```

## 🚀 사용법

### 기본 사용

```bash
# 기본 사용법: 입력 폴더 지정
jconvert -i ./data

# 출력 파일명 지정
jconvert -i ./data -o result.jsonl

# 상세 출력 모드
jconvert -i ./data -o result.jsonl --verbose
```

### 고급 옵션

```bash
# 기존 파일에 추가
jconvert -i ./data -o result.jsonl --mode append

# 패턴 필터링 (와일드카드 지원)
jconvert -i ./data -o result.jsonl --pattern "*_SUM_*"

# 드라이런 모드 (실제 병합 없이 파일 목록만 확인)
jconvert -i ./data --dry-run

# 스레드 수 지정
jconvert -i ./data -o result.jsonl -j 4
```

### 전체 옵션

```
옵션:
  -i, --input <INPUT>      JSON 파일들이 있는 입력 폴더 경로
  -o, --output <OUTPUT>    생성될 JSONL 파일 경로 [기본값: output.jsonl]
  -m, --mode <MODE>        출력 파일 모드 [가능한 값: overwrite, append, error]
  -p, --pattern <PATTERN>  파일 이름 패턴 필터 (예: "*_SUM_*")
  -v, --verbose            상세 출력 모드
      --dry-run            실제 병합 없이 처리될 파일 목록만 표시
  -j, --threads <THREADS>  병렬 처리 스레드 수 (기본값: CPU 코어 수)
  -h, --help               도움말 표시
  -V, --version            버전 정보 표시
```

## 📊 출력 예시

```
══════════════════════════════════════════════════
 🚀 JSON FOLDER TO JSONL CONVERTER
══════════════════════════════════════════════════
  📂 입력 폴더: "./VL_해석례_SUM"
  📄 출력 파일: "output.jsonl"
  ⚙️ 모드: Overwrite
══════════════════════════════════════════════════

📁 파일 검색 중...
  📋 발견된 파일 수: 6

⚡ 병렬 처리 중...
🟢 [00:00:00] [████████████████████████████████████████] 6/6 (100%) 완료!

💾 JSONL 파일 저장 중...

══════════════════════════════════════════════════
 📊 처리 통계
══════════════════════════════════════════════════
  📁 전체 파일:    6
  ✅ 성공:         6
  ✅ 실패:         0
  📥 입력 용량:    10.11 KB
  📤 출력 용량:    4.87 KB
  📈 성공률:       100.0%
══════════════════════════════════════════════════

✅ 저장 완료: "output.jsonl"
```

## 🔧 개발

### 요구 사항

- Rust 1.70 이상
- Cargo

### 테스트 실행

```bash
cargo test
```

### 린트 검사

```bash
cargo clippy
```

## 📁 프로젝트 구조

```
jsonl_con/
├── Cargo.toml       # 프로젝트 설정 및 의존성
├── README.md        # 이 문서
└── src/
    └── main.rs      # 메인 소스 코드
```

## 📄 라이선스

MIT License
