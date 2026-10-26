//! Korean interface (USeop9614/deckcraft patch).
//!
//! The interface strings stay English in the code. [`install`] gives egui a translator, so every
//! plain string a widget, tooltip, window title or painted label shows is looked up in
//! `i18n/ko.tsv` (embedded at build time) and drawn in Korean when there is a row for it.
//!
//! - Exact rows: `English<TAB>한국어`. `\n` and `\t` escapes stand for newline and tab.
//! - Template rows: the English side has `{…}` placeholders (`Slide {} of {n}`). They match any
//!   text in their place; the Korean side refers back to them by name (`{n}`), by position
//!   (`{1}`, `{2}` …) or in order (`{}`).
//! - A string ending in `…` is translated from its row without the ellipsis when it has none.
//!
//! Set `DECKCRAFT_LANG=en` to keep the English interface.

use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};

const CATALOG: &str = include_str!("../i18n/ko.tsv");

/// Cached derived translations (templates, ellipsis forms) and misses are bounded so text that
/// changes every frame (zoom percentages, times) cannot grow memory without limit.
const MAX_CACHE: usize = 20_000;

struct Template {
    /// Literal pieces around the placeholders; `parts.len() == names.len() + 1`.
    parts: Vec<String>,
    /// Placeholder names (`""` for `{}` or a bare format spec such as `{:.1}`).
    names: Vec<String>,
    value: &'static str,
}

struct Catalog {
    exact: HashMap<&'static str, &'static str>,
    templates: Vec<Template>,
    /// Every Korean string, for the interface fonts to cover.
    all_text: String,
}

#[derive(Default)]
struct Cache {
    hits: HashMap<String, &'static str>,
    misses: HashSet<String>,
}

static CAT: LazyLock<Catalog> = LazyLock::new(|| parse(CATALOG));
static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(|| Mutex::new(Cache::default()));
static ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some(o) => {
                    out.push('\\');
                    out.push(o);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// Split `Slide {} of {n}` into literal parts and placeholder names. None when there is no
/// placeholder.
fn split_template(key: &str) -> Option<(Vec<String>, Vec<String>)> {
    let mut parts = vec![String::new()];
    let mut names = Vec::new();
    let mut rest = key;
    while let Some(open) = rest.find('{') {
        let Some(close_rel) = rest[open..].find('}') else { break };
        let close = open + close_rel;
        let inner = &rest[open + 1..close];
        if let Some(last) = parts.last_mut() {
            last.push_str(&rest[..open]);
        }
        names.push(inner.split(':').next().unwrap_or("").to_string());
        parts.push(String::new());
        rest = &rest[close + 1..];
    }
    if let Some(last) = parts.last_mut() {
        last.push_str(rest);
    }
    if names.is_empty() { None } else { Some((parts, names)) }
}

fn parse(text: &'static str) -> Catalog {
    let mut exact = HashMap::new();
    let mut templates = Vec::new();
    let mut all_text = String::new();
    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let Some((en, ko)) = line.split_once('\t') else { continue };
        if en.is_empty() || ko.is_empty() {
            continue;
        }
        let en = unescape(en);
        let ko = unescape(ko);
        all_text.push_str(&ko);
        let ko: &'static str = leak(ko);
        if let Some((parts, names)) = split_template(&en) {
            // A template whose literal text is empty would match anything.
            if parts.iter().any(|p| !p.trim().is_empty()) {
                templates.push(Template { parts, names, value: ko });
            }
        } else {
            exact.insert(&*leak(en), ko);
        }
    }
    // Longer literal text first, so `Slide {} of {}` wins over `Slide {}`.
    templates.sort_by_key(|t| std::cmp::Reverse(t.parts.iter().map(String::len).sum::<usize>()));
    Catalog { exact, templates, all_text }
}

/// Match `s` against a template, returning the captured pieces.
fn capture<'a>(t: &Template, s: &'a str) -> Option<Vec<&'a str>> {
    let first = t.parts.first()?;
    let mut rest = s.strip_prefix(first.as_str())?;
    let mut caps = Vec::with_capacity(t.names.len());
    for (i, part) in t.parts.iter().enumerate().skip(1) {
        let last = i == t.parts.len() - 1;
        if last {
            if part.is_empty() {
                caps.push(rest);
                rest = "";
            } else {
                let cap = rest.strip_suffix(part.as_str())?;
                caps.push(cap);
                rest = "";
            }
        } else if part.is_empty() {
            // Two placeholders side by side: give the first one nothing it can't do without.
            caps.push("");
        } else {
            let at = rest.find(part.as_str())?;
            caps.push(&rest[..at]);
            rest = &rest[at + part.len()..];
        }
    }
    if !rest.is_empty() {
        return None;
    }
    // An unnamed placeholder at the very end (`Slide {}`) stands for a number: letting it take
    // words would half-translate other strings that merely start the same way (`Slide Show`).
    let open_end = t.parts.last().is_some_and(|p| p.trim().is_empty());
    if open_end {
        if let (Some(name), Some(cap)) = (t.names.last(), caps.last()) {
            if name.is_empty() && cap.chars().any(char::is_alphabetic) {
                return None;
            }
        }
    }
    Some(caps)
}

fn fill(t: &Template, caps: &[&str]) -> String {
    let v = t.value;
    let mut out = String::with_capacity(v.len() + 16);
    let mut seq = 0usize;
    let mut rest = v;
    while let Some(open) = rest.find('{') {
        let Some(close_rel) = rest[open..].find('}') else { break };
        let close = open + close_rel;
        out.push_str(&rest[..open]);
        let inner = &rest[open + 1..close];
        let name = inner.split(':').next().unwrap_or("");
        let piece = if name.is_empty() {
            let p = caps.get(seq).copied();
            seq += 1;
            p
        } else if let Ok(n) = name.parse::<usize>() {
            n.checked_sub(1).and_then(|i| caps.get(i)).copied()
        } else {
            t.names.iter().position(|x| x == name).and_then(|i| caps.get(i)).copied()
        };
        match piece {
            Some(p) => out.push_str(p),
            None => out.push_str(&rest[open..=close]),
        }
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    out
}

fn derive(cat: &Catalog, s: &str) -> Option<String> {
    for t in &cat.templates {
        if let Some(caps) = capture(t, s) {
            return Some(fill(t, &caps));
        }
    }
    if let Some(base) = s.strip_suffix('…').or_else(|| s.strip_suffix("...")) {
        if let Some(k) = cat.exact.get(base) {
            return Some(format!("{k}…"));
        }
    }
    // Leading/trailing spaces around a known string ("  Bold ").
    let trimmed = s.trim();
    if trimmed.len() != s.len() && !trimmed.is_empty() {
        if let Some(k) = cat.exact.get(trimmed) {
            return Some(s.replacen(trimmed, k, 1));
        }
    }
    None
}

/// The Korean text for an interface string, if the catalog has it.
pub fn lookup(s: &str) -> Option<&'static str> {
    if s.is_empty() || !s.bytes().any(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    let cat = &*CAT;
    if let Some(k) = cat.exact.get(s) {
        return Some(k);
    }
    let Ok(mut cache) = CACHE.lock() else { return None };
    if let Some(k) = cache.hits.get(s) {
        return Some(k);
    }
    if cache.misses.contains(s) {
        return None;
    }
    match derive(cat, s) {
        Some(t) if cache.hits.len() < MAX_CACHE => {
            let t = leak(t);
            cache.hits.insert(s.to_string(), t);
            Some(t)
        }
        Some(_) => None,
        None => {
            if cache.misses.len() < MAX_CACHE * 4 {
                cache.misses.insert(s.to_string());
            }
            None
        }
    }
}

/// `s` in Korean when the Korean interface is on and the catalog has it, else `s` unchanged. For
/// text that is measured before it is painted (ribbon labels, tabs).
pub fn tr(s: &str) -> &str {
    if active() { lookup(s).unwrap_or(s) } else { s }
}

/// Turn the Korean interface on (unless `DECKCRAFT_LANG=en`). Call once, before the first frame.
pub fn install() {
    if std::env::var("DECKCRAFT_LANG").is_ok_and(|v| v.trim().eq_ignore_ascii_case("en")) {
        return;
    }
    LazyLock::force(&CAT);
    egui::set_translator(lookup);
    ACTIVE.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Is the Korean interface on?
pub fn active() -> bool {
    ACTIVE.load(std::sync::atomic::Ordering::Relaxed)
}

/// Every Korean string in the catalog, so the interface fonts can borrow a Hangul face up front.
pub fn catalog_text() -> &'static str {
    &CAT.all_text
}

static FONTS_NOTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The catalog text the first time it is asked for while Korean is on; None afterwards.
pub fn take_font_text() -> Option<&'static str> {
    if !active() || FONTS_NOTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return None;
    }
    Some(catalog_text())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_template_and_ellipsis() {
        let cat = parse("Insert\t삽입\nSlide {} of {n}\t슬라이드 {}/{n}\nMatch {} of {} here\t{2}개 중 {1}\nNew\\nSlide\t새\\n슬라이드\n");
        assert_eq!(cat.exact.get("Insert"), Some(&"삽입"));
        assert_eq!(cat.exact.get("New\nSlide"), Some(&"새\n슬라이드"));
        assert_eq!(derive(&cat, "Slide 3 of 10").as_deref(), Some("슬라이드 3/10"));
        assert_eq!(derive(&cat, "Match 2 of 5 here").as_deref(), Some("5개 중 2"));
        assert_eq!(derive(&cat, "Insert…").as_deref(), Some("삽입…"));
        assert_eq!(derive(&cat, "Something else"), None);
    }

    #[test]
    fn embedded_catalog_parses() {
        assert!(CAT.exact.len() > 1000);
        assert_eq!(CAT.exact.get("Home"), Some(&"홈"));
    }
}
