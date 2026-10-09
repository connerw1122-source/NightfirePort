//! Golden-snapshot comparison for tab-separated tables keyed by their first column. Snapshots hold numbers,
//! identifiers and SHA-1s only, never game bytes (CLAUDE.md).

use std::collections::BTreeMap;

/// Differences between two TSV tables (first line = header), keyed by column 0: rows missing on either side and
/// rows whose other columns differ. Empty means identical as sets of rows.
pub fn diff_tsv(expected: &str, actual: &str) -> Vec<String> {
    fn rows(s: &str) -> (Option<&str>, BTreeMap<&str, &str>) {
        let mut it = s.lines().filter(|l| !l.is_empty());
        let header = it.next();
        (header, it.map(|l| l.split_once('\t').unwrap_or((l, ""))).collect())
    }
    let ((eh, e), (ah, a)) = (rows(expected), rows(actual));
    let mut out = Vec::new();
    if eh != ah { out.push(format!("header: expected {eh:?}, got {ah:?}")); }
    for (k, v) in &e {
        match a.get(k) {
            None => out.push(format!("missing: {k}")),
            Some(w) if w != v => out.push(format!("changed: {k}: expected {v:?}, got {w:?}")),
            _ => {}
        }
    }
    for k in a.keys().filter(|k| !e.contains_key(*k)) { out.push(format!("unexpected: {k}")); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: &str = "path\tlba\tsize\n/A\t1\t10\n/B\t2\t20\n";

    #[test]
    fn identical_tables_have_no_differences() {
        assert!(diff_tsv(T, T).is_empty());
        assert!(diff_tsv(T, "path\tlba\tsize\n/B\t2\t20\n/A\t1\t10\n").is_empty(), "row order must not matter");
    }

    /// Sister test: every kind of mismatch is reported.
    #[test]
    fn every_kind_of_difference_is_reported() {
        assert_eq!(diff_tsv(T, "path\tlba\tsize\n/A\t1\t11\n/B\t2\t20\n"), ["changed: /A: expected \"1\\t10\", got \"1\\t11\""]);
        assert_eq!(diff_tsv(T, "path\tlba\tsize\n/A\t1\t10\n"), ["missing: /B"]);
        assert_eq!(diff_tsv(T, &format!("{T}/C\t3\t30\n")), ["unexpected: /C"]);
        assert_eq!(diff_tsv(T, &T.replace("size", "len")).len(), 1);
    }
}
