//! 패턴 매칭 모듈
//!
//! glob 패턴을 사용한 파일 이름 필터링을 담당합니다.

use glob::Pattern;

use crate::error::{JConvertError, Result};

/// 컴파일된 패턴 매처
#[derive(Default)]
pub struct PatternMatcher {
    pattern: Option<Pattern>,
}

impl PatternMatcher {
    /// 새 패턴 매처 생성
    ///
    /// # Arguments
    /// * `pattern` - 글로브 패턴 문자열 (None이면 모든 파일 매칭)
    ///
    /// # Returns
    /// 컴파일된 `PatternMatcher` 또는 에러
    ///
    /// # Examples
    /// ```
    /// use jconvert::pattern::PatternMatcher;
    ///
    /// let matcher = PatternMatcher::new(Some("*_SUM_*".to_string())).unwrap();
    /// assert!(matcher.matches("test_SUM_1.json"));
    /// assert!(!matcher.matches("other.json"));
    /// ```
    pub fn new(pattern: Option<String>) -> Result<Self> {
        let compiled = match pattern {
            Some(ref p) => Some(
                Pattern::new(p)
                    .map_err(|_| JConvertError::InvalidPattern { pattern: p.clone() })?,
            ),
            None => None,
        };

        Ok(Self { pattern: compiled })
    }

    /// 파일 이름이 패턴과 일치하는지 확인
    ///
    /// # Arguments
    /// * `file_name` - 검사할 파일 이름
    ///
    /// # Returns
    /// 패턴 일치 여부 (패턴이 없으면 항상 true)
    pub fn matches(&self, file_name: &str) -> bool {
        match &self.pattern {
            Some(p) => p.matches(file_name),
            None => true,
        }
    }

    /// 패턴이 설정되어 있는지 확인
    pub fn has_pattern(&self) -> bool {
        self.pattern.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matcher_with_wildcard() {
        let matcher = PatternMatcher::new(Some("*_SUM_*".to_string())).unwrap();
        assert!(matcher.matches("test_SUM_1.json"));
        assert!(matcher.matches("HS_H_323503_SUM_15.json"));
        assert!(!matcher.matches("test.json"));
        assert!(!matcher.matches("SUM.json"));
    }

    #[test]
    fn test_pattern_matcher_with_question_mark() {
        let matcher = PatternMatcher::new(Some("data?.json".to_string())).unwrap();
        assert!(matcher.matches("data1.json"));
        assert!(matcher.matches("dataA.json"));
        assert!(!matcher.matches("data.json"));
        assert!(!matcher.matches("data12.json"));
    }

    #[test]
    fn test_pattern_matcher_with_brackets() {
        let matcher = PatternMatcher::new(Some("file[0-9].json".to_string())).unwrap();
        assert!(matcher.matches("file1.json"));
        assert!(matcher.matches("file9.json"));
        assert!(!matcher.matches("fileA.json"));
    }

    #[test]
    fn test_pattern_matcher_none() {
        let matcher = PatternMatcher::new(None).unwrap();
        assert!(matcher.matches("anything.json"));
        assert!(matcher.matches("test_SUM_1.json"));
    }

    #[test]
    fn test_pattern_matcher_invalid() {
        let result = PatternMatcher::new(Some("[invalid".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_has_pattern() {
        let with_pattern = PatternMatcher::new(Some("*.json".to_string())).unwrap();
        let without_pattern = PatternMatcher::new(None).unwrap();

        assert!(with_pattern.has_pattern());
        assert!(!without_pattern.has_pattern());
    }
}
