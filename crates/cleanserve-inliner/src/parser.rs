use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EnvError {
    #[error("Failed to read .env: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Default)]
pub struct EnvEntry {
    pub key: String,
    pub value: String,
    pub comment: Option<String>,
}

pub struct EnvParser {
    entries: Vec<EnvEntry>,
}

impl EnvParser {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, EnvError> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::from_str(&content))
    }

    pub fn from_str(content: &str) -> Self {
        let mut entries = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with('#') {
                if let Some(comment) = trimmed.strip_prefix('#') {
                    entries.push(EnvEntry {
                        key: String::new(),
                        value: String::new(),
                        comment: Some(comment.trim().to_string()),
                    });
                }
                continue;
            }

            if let Some(pos) = trimmed.find('=') {
                let key = trimmed[..pos].trim().to_string();
                let mut value = trimmed[pos + 1..].trim().to_string();

                if (value.starts_with('"') && value.ends_with('"'))
                    || (value.starts_with('\'') && value.ends_with('\''))
                {
                    value = value[1..value.len() - 1].to_string();
                }

                entries.push(EnvEntry {
                    key,
                    value,
                    comment: None,
                });
            }
        }

        Self { entries }
    }

    pub fn to_map(&self) -> HashMap<String, String> {
        self.entries
            .iter()
            .filter(|e| !e.key.is_empty())
            .map(|e| (e.key.clone(), e.value.clone()))
            .collect()
    }

    pub fn keys(&self) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|e| !e.key.is_empty())
            .map(|e| e.key.as_str())
            .collect()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.key == key)
            .map(|e| e.value.as_str())
    }

    pub fn entries(&self) -> &[EnvEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.iter().filter(|e| !e.key.is_empty()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl IntoIterator for EnvParser {
    type Item = EnvEntry;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_env() {
        let parser = EnvParser::from_str(
            r#"
APP_NAME=MyApp
APP_ENV=production
API_KEY=secret123
"#,
        );

        assert_eq!(parser.len(), 3);
        assert_eq!(parser.get("APP_NAME"), Some("MyApp"));
        assert_eq!(parser.get("APP_ENV"), Some("production"));
        assert_eq!(parser.get("API_KEY"), Some("secret123"));
    }

    #[test]
    fn test_parse_quoted_values() {
        let parser = EnvParser::from_str(
            r#"
DB_HOST="localhost"
DB_PASS='secret'
"#,
        );

        assert_eq!(parser.get("DB_HOST"), Some("localhost"));
        assert_eq!(parser.get("DB_PASS"), Some("secret"));
    }

    #[test]
    fn test_parse_with_comments() {
        let parser = EnvParser::from_str(
            r#"
# Database config
DB_HOST=localhost

# API Keys
API_KEY=key123
"#,
        );

        assert_eq!(parser.len(), 2);
        assert_eq!(parser.get("DB_HOST"), Some("localhost"));
        assert_eq!(parser.get("API_KEY"), Some("key123"));
    }

    #[test]
    fn test_empty_value() {
        let parser = EnvParser::from_str("EMPTY_KEY=");
        assert_eq!(parser.get("EMPTY_KEY"), Some(""));
    }

    #[test]
    fn test_to_map() {
        let parser = EnvParser::from_str("A=1\nB=2");
        let map = parser.to_map();

        assert_eq!(map.get("A"), Some(&"1".to_string()));
        assert_eq!(map.get("B"), Some(&"2".to_string()));
    }
}
