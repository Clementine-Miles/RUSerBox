//! Localized strings: the Flutter app's ARB tables (its own and fl_lib's),
//! converted to `app/l10n/<lang>.json` with the removed features' strings
//! dropped. A key missing in the current language falls back to English, and
//! one missing there to the text the call site gives.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

macro_rules! tables {
    ($($lang:literal),*) => {
        &[$(($lang, include_str!(concat!("../l10n/", $lang, ".json")))),*]
    };
}

const TABLES: &[(&str, &str)] = tables!("en", "az", "de", "es", "fa", "fr", "id", "it", "ja", "ko", "nl", "pt", "ru", "tr", "uk", "zh", "zh_tw");

type Table = HashMap<String, String>;

fn parse(json: &str) -> Table {
    serde_json::from_str::<HashMap<String, serde_json::Value>>(json)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_owned())))
        .collect()
}

fn english() -> &'static Table {
    static EN: OnceLock<Table> = OnceLock::new();
    EN.get_or_init(|| parse(TABLES[0].1))
}

static CURRENT: RwLock<Option<(String, Table)>> = RwLock::new(None);

/// The languages there are tables for.
pub fn languages() -> impl Iterator<Item = &'static str> {
    TABLES.iter().map(|(l, _)| *l)
}

/// Picks the table for a locale such as `zh-Hant-TW`, `pt_BR` or `de`.
pub fn set_locale(locale: &str) {
    let norm = locale.replace('-', "_").to_lowercase();
    let want = if norm.starts_with("zh") && (norm.contains("tw") || norm.contains("hk") || norm.contains("hant")) { "zh_tw".to_owned() } else { norm.split('_').next().unwrap_or("en").to_owned() };
    let found = TABLES.iter().find(|(l, _)| *l == want).map(|(l, j)| ((*l).to_owned(), parse(j)));
    *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = found;
}

pub fn locale() -> String {
    CURRENT.read().unwrap_or_else(|e| e.into_inner()).as_ref().map(|(l, _)| l.clone()).unwrap_or_else(|| "en".into())
}

/// The string for [key], else [fallback].
pub fn t(key: &str, fallback: &str) -> String {
    if let Some((_, table)) = CURRENT.read().unwrap_or_else(|e| e.into_inner()).as_ref()
        && let Some(s) = table.get(key)
    {
        return s.clone();
    }
    english().get(key).cloned().unwrap_or_else(|| fallback.to_owned())
}

/// [t] with `{name}` placeholders filled.
pub fn tf(key: &str, fallback: &str, args: &[(&str, &str)]) -> String {
    let mut s = t(key, fallback);
    for (k, v) in args {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locales_and_fallbacks() {
        set_locale("de_DE");
        assert_eq!(locale(), "de");
        assert_ne!(t("cancel", "Cancel"), "");
        assert_eq!(t("noSuchKeyAnywhere", "Fallback"), "Fallback");
        set_locale("zh-Hant-TW");
        assert_eq!(locale(), "zh_tw");
        set_locale("xx");
        assert_eq!(locale(), "en");
        assert_eq!(t("cancel", "?"), "Cancel");
        for (lang, json) in TABLES {
            assert!(!parse(json).is_empty(), "{lang}");
            assert!(!parse(json).keys().any(|k| k.starts_with("agent") || k.starts_with("askAi")), "{lang} carries AI strings");
        }
    }
}
