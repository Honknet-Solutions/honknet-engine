use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;
#[derive(Debug, Error)]
pub enum LocalizationError {
    #[error("invalid FTL line {0}")]
    Line(usize),
    #[error("missing message {0}")]
    Missing(String),
}

#[derive(Default, Clone)]
pub struct Localization {
    locales: Arc<RwLock<HashMap<String, HashMap<String, String>>>>,
    fallback: Arc<RwLock<String>>,
}

impl Localization {
    pub fn set_fallback(&self, l: impl Into<String>) {
        *self.fallback.write() = l.into()
    }
    pub fn load_ftl(&self, locale: &str, text: &str) -> Result<usize, LocalizationError> {
        let mut map = self.locales.write();
        let m = map.entry(locale.into()).or_default();
        let mut n = 0;
        let mut current: Option<String> = None;
        for (i, line) in text.lines().enumerate() {
            let line = line.trim_end();
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            if line.starts_with(' ') || line.starts_with('\t') {
                if let Some(k) = &current {
                    m.entry(k.clone()).and_modify(|v| {
                        v.push('\n');
                        v.push_str(line.trim())
                    });
                    continue;
                } else {
                    return Err(LocalizationError::Line(i + 1));
                }
            }
            let Some((k, v)) = line.split_once('=') else {
                return Err(LocalizationError::Line(i + 1));
            };
            let k = k.trim().to_string();
            m.insert(k.clone(), v.trim().to_string());
            current = Some(k);
            n += 1
        }
        Ok(n)
    }
    pub fn format(
        &self,
        locale: &str,
        key: &str,
        args: &HashMap<String, String>,
    ) -> Result<String, LocalizationError> {
        let maps = self.locales.read();
        let fallback = self.fallback.read();
        let mut v = maps
            .get(locale)
            .and_then(|m| m.get(key))
            .or_else(|| maps.get(&*fallback).and_then(|m| m.get(key)))
            .cloned()
            .ok_or_else(|| LocalizationError::Missing(key.into()))?;
        for (k, x) in args {
            v = v.replace(&format!("{{ ${k} }}"), x)
        }
        Ok(v)
    }
}
