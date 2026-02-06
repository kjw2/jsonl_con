//! 통계 및 유틸리티 모듈
//!
//! 처리 통계 수집 및 포맷팅을 담당합니다.

use colored::Colorize;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// 처리 통계 구조체
#[derive(Debug, Default)]
pub struct Statistics {
    /// 총 파일 수
    pub total_files: usize,
    /// 성공 처리 수
    pub success_count: AtomicUsize,
    /// 에러 발생 수
    pub error_count: AtomicUsize,
    /// 읽은 총 바이트
    pub total_bytes_read: AtomicU64,
    /// 쓴 총 바이트
    pub total_bytes_written: AtomicU64,
    /// 유효성 검사 실패 수
    pub validation_failed: AtomicUsize,
    /// 처리 시작 시간
    start_time: Option<Instant>,
}

impl Statistics {
    /// 새 통계 인스턴스 생성
    pub fn new(total_files: usize) -> Self {
        Self {
            total_files,
            start_time: Some(Instant::now()),
            ..Default::default()
        }
    }

    /// 성공 카운트 증가
    pub fn increment_success(&self) {
        self.success_count.fetch_add(1, Ordering::Relaxed);
    }

    /// 에러 카운트 증가
    pub fn increment_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    /// 유효성 검사 실패 카운트 증가
    pub fn increment_validation_failed(&self) {
        self.validation_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// 읽은 바이트 추가
    pub fn add_bytes_read(&self, bytes: u64) {
        self.total_bytes_read.fetch_add(bytes, Ordering::Relaxed);
    }

    /// 쓴 바이트 추가
    pub fn add_bytes_written(&self, bytes: u64) {
        self.total_bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }

    /// 성공 수 반환
    pub fn get_success_count(&self) -> usize {
        self.success_count.load(Ordering::Relaxed)
    }

    /// 에러 수 반환
    pub fn get_error_count(&self) -> usize {
        self.error_count.load(Ordering::Relaxed)
    }

    /// 유효성 검사 실패 수 반환
    pub fn get_validation_failed(&self) -> usize {
        self.validation_failed.load(Ordering::Relaxed)
    }

    /// 경과 시간 반환
    pub fn elapsed(&self) -> Duration {
        self.start_time
            .map(|t| t.elapsed())
            .unwrap_or(Duration::ZERO)
    }

    /// 일반 처리 통계 요약 출력
    pub fn print_summary(&self) {
        let success = self.get_success_count();
        let errors = self.get_error_count();
        let bytes_read = self.total_bytes_read.load(Ordering::Relaxed);
        let bytes_written = self.total_bytes_written.load(Ordering::Relaxed);
        let elapsed = self.elapsed();

        println!("\n{}", "═".repeat(50).bright_blue());
        println!("{}", " 📊 처리 통계".bright_white().bold());
        println!("{}", "═".repeat(50).bright_blue());

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

        println!(
            "  {} 처리 시간:    {:.2}초",
            "⏱️".bright_cyan(),
            elapsed.as_secs_f64()
        );

        println!("{}", "═".repeat(50).bright_blue());
    }

    /// 유효성 검사 통계 요약 출력
    pub fn print_validation_summary(&self) {
        let success = self.get_success_count();
        let failed = self.get_validation_failed();
        let elapsed = self.elapsed();

        println!("\n{}", "═".repeat(50).bright_blue());
        println!("{}", " 🔍 유효성 검사 결과".bright_white().bold());
        println!("{}", "═".repeat(50).bright_blue());

        println!(
            "  {} 전체 파일:    {}",
            "📁".bright_cyan(),
            self.total_files
        );
        println!(
            "  {} 유효:         {}",
            "✅".bright_green(),
            success.to_string().green()
        );

        if failed > 0 {
            println!(
                "  {} 무효:         {}",
                "❌".bright_red(),
                failed.to_string().red()
            );
        } else {
            println!("  {} 무효:         {}", "✅".bright_green(), "0".green());
        }

        if self.total_files > 0 {
            let valid_rate = (success as f64 / self.total_files as f64) * 100.0;
            println!("  {} 유효율:       {:.1}%", "📈".bright_white(), valid_rate);
        }

        println!(
            "  {} 검사 시간:    {:.2}초",
            "⏱️".bright_cyan(),
            elapsed.as_secs_f64()
        );

        println!("{}", "═".repeat(50).bright_blue());
    }
}

/// 바이트를 읽기 쉬운 형식으로 변환
///
/// # Arguments
/// * `bytes` - 바이트 수
///
/// # Returns
/// 형식화된 문자열 (예: "1.25 MB")
///
/// # Examples
/// ```
/// use jconvert::stats::format_bytes;
///
/// assert_eq!(format_bytes(500), "500 B");
/// assert_eq!(format_bytes(1024), "1.00 KB");
/// assert_eq!(format_bytes(1048576), "1.00 MB");
/// ```
pub fn format_bytes(bytes: u64) -> String {
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

/// 경과 시간을 읽기 쉬운 형식으로 변환
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();

    if secs >= 3600 {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        format!("{}시간 {}분", hours, mins)
    } else if secs >= 60 {
        let mins = secs / 60;
        let remaining_secs = secs % 60;
        format!("{}분 {}초", mins, remaining_secs)
    } else if secs > 0 {
        format!("{}.{:03}초", secs, millis)
    } else {
        format!("{}ms", millis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration(Duration::from_secs(5)), "5.000초");
        assert_eq!(format_duration(Duration::from_secs(65)), "1분 5초");
        assert_eq!(format_duration(Duration::from_secs(3665)), "1시간 1분");
    }

    #[test]
    fn test_statistics_counters() {
        let stats = Statistics::new(10);

        stats.increment_success();
        stats.increment_success();
        stats.increment_error();
        stats.add_bytes_read(1024);
        stats.add_bytes_written(512);

        assert_eq!(stats.get_success_count(), 2);
        assert_eq!(stats.get_error_count(), 1);
        assert_eq!(stats.total_bytes_read.load(Ordering::Relaxed), 1024);
        assert_eq!(stats.total_bytes_written.load(Ordering::Relaxed), 512);
    }
}
