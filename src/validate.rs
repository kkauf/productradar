//! Data validation. Errors fail the build; warnings are printed. This is the test suite for content.

use crate::model::*;
use std::collections::{HashMap, HashSet};

pub struct Report {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

fn is_slug(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') && !s.starts_with('-') && !s.ends_with('-')
}

fn is_url(s: &str) -> bool {
    s.starts_with("https://") && s.len() > 8 && !s.contains(char::is_whitespace)
}

/// Private terms that must never reach a public page: one per line in the file at env
/// `RADAR_DENYLIST`, else `~/.claude/product-radar-data/denylist.txt`. `#` starts a comment.
/// The file is private and never committed; a missing file means no private terms.
pub fn load_denylist() -> Vec<String> {
    let path = std::env::var_os("RADAR_DENYLIST")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".claude/product-radar-data/denylist.txt")));
    let Some(text) = path.and_then(|p| std::fs::read_to_string(p).ok()) else { return Vec::new() };
    parse_denylist(&text)
}

fn parse_denylist(text: &str) -> Vec<String> {
    text.lines().map(|l| l.split('#').next().unwrap_or("").trim()).filter(|l| !l.is_empty()).map(str::to_string).collect()
}

/// Longest public rationale before a warning: articles stay short.
pub const RATIONALE_MAX: usize = 900;

fn word_char(c: Option<char>) -> bool {
    c.is_some_and(char::is_alphanumeric)
}

/// `term` occurs in `text`, case-insensitively, with no letter or digit right before or after it.
/// Runs of whitespace count as one space, as on the rendered page, so a line break inside a
/// two-word term still matches.
fn has_term(text: &str, term: &str) -> bool {
    let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let (t, w) = (norm(text), norm(term));
    if w.is_empty() {
        return false;
    }
    t.match_indices(&w).any(|(i, _)| !word_char(t[..i].chars().next_back()) && !word_char(t[i + w.len()..].chars().next()))
}

/// An email address: a local part, `@`, and a domain with a dot.
fn has_email(text: &str) -> bool {
    let local = |c: char| c.is_alphanumeric() || "._%+-".contains(c);
    let domain = |c: char| c.is_alphanumeric() || ".-".contains(c);
    text.match_indices('@').any(|(i, _)| {
        let before = text[..i].chars().next_back().is_some_and(local);
        let dom: String = text[i + 1..].chars().take_while(|&c| domain(c)).collect();
        let dom = dom.trim_end_matches(['.', '-']);
        before
            && dom.split('.').count() >= 2
            && dom.split('.').all(|p| !p.is_empty())
            && dom.rsplit('.').next().is_some_and(|tld| tld.len() >= 2 && tld.chars().all(char::is_alphabetic))
    })
}

/// Internal material in a public text field: one description per kind found.
fn leak(text: &str, deny: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    if has_email(text) {
        out.push("an email address".to_string());
    }
    let lower = text.to_lowercase();
    for p in ["/users/", "~/", "localhost", "127.0.0.1"] {
        if lower.contains(p) {
            out.push(format!("`{p}`"));
        }
    }
    if lower.match_indices(".env").any(|(i, _)| !word_char(lower[i + 4..].chars().next())) {
        out.push("`.env`".to_string());
    }
    for term in deny {
        if has_term(text, term) {
            out.push(format!("the private term `{term}`"));
        }
    }
    out
}

/// Every public text field of every radar and entry, as (file, field, text).
fn public_texts(d: &Data) -> Vec<(String, String, &str)> {
    let mut v: Vec<(String, String, &str)> = Vec::new();
    for e in &d.entries {
        let at = format!("entries/{}.toml", e.id);
        v.push((at.clone(), "name".into(), &e.name));
        v.push((at.clone(), "summary".into(), &e.summary));
        if let Some(l) = &e.label {
            v.push((at.clone(), "label".into(), l));
        }
        for ev in &e.evidence {
            v.push((at.clone(), format!("evidence claim for {}", ev.url), &ev.claim));
        }
    }
    for r in &d.radars {
        let at = format!("radars/{}.toml", r.id);
        v.push((at.clone(), "title".into(), &r.title));
        v.push((at.clone(), "description".into(), &r.description));
        for ed in &r.editions {
            v.push((at.clone(), format!("edition {} title", ed.date), &ed.title));
            v.push((at.clone(), format!("edition {} note", ed.date), &ed.note));
            if let Some(b) = &ed.body {
                v.push((at.clone(), format!("edition {} body", ed.date), b));
            }
        }
        for b in &r.blips {
            v.push((at.clone(), format!("blip `{}` rationale", b.entry), &b.rationale));
            for m in &b.history {
                v.push((at.clone(), format!("blip `{}` history note on {}", b.entry, m.date), &m.note));
            }
        }
        for rj in &r.rejected {
            v.push((at.clone(), "rejected name".into(), &rj.name));
            v.push((at.clone(), format!("rejected `{}` reason", rj.name), &rj.reason));
        }
    }
    v
}

/// Extra errors for `check --ship`: an edition ships only when every curated blip is confirmed.
pub fn ship_errors(d: &Data) -> Vec<String> {
    let mut err = Vec::new();
    for r in d.radars.iter().filter(|r| r.kind == Kind::Curated) {
        for b in r.blips.iter().filter(|b| b.status == Status::Draft) {
            err.push(format!("radars/{}.toml: blip `{}` has status = \"draft\"; --ship needs every curated blip confirmed", r.id, b.entry));
        }
    }
    err
}

/// Validation with no private terms. Callers that ship use `validate_with(d, &load_denylist())`.
#[allow(dead_code)]
pub fn validate(d: &Data) -> Report {
    validate_with(d, &[])
}

pub fn validate_with(d: &Data, deny: &[String]) -> Report {
    let mut err = Vec::new();
    let mut warn = Vec::new();
    let s = &d.site;

    // Site: exactly four quadrants on distinct corners, exactly four rings. The visual system assumes both.
    if s.quadrants.len() != 4 {
        err.push(format!("site.toml: expected 4 quadrants, found {}", s.quadrants.len()));
    }
    let corners: HashSet<_> = s.quadrants.iter().map(|q| q.position as u8).collect();
    if corners.len() != s.quadrants.len() {
        err.push("site.toml: two quadrants share a position".into());
    }
    if s.rings.len() != 4 {
        err.push(format!("site.toml: expected 4 rings, found {}", s.rings.len()));
    }
    let mut seen = HashSet::new();
    for id in s.quadrants.iter().map(|q| &q.id).chain(s.rings.iter().map(|r| &r.id)) {
        if !is_slug(id) {
            err.push(format!("site.toml: id `{id}` must be lowercase-kebab-case"));
        }
        if !seen.insert(id.as_str()) || id == RETIRED {
            err.push(format!("site.toml: id `{id}` is duplicated or reserved"));
        }
    }
    if !s.base_url.starts_with("https://") || s.base_url.ends_with('/') {
        err.push("site.toml: base_url must start with https:// and have no trailing slash".into());
    }
    let quads: HashSet<&str> = s.quadrants.iter().map(|q| q.id.as_str()).collect();
    let rings: HashSet<&str> = s.rings.iter().map(|r| r.id.as_str()).collect();

    // Entries: facts about a thing, independent of any radar.
    let mut entries = HashMap::new();
    for e in &d.entries {
        let at = format!("entries/{}.toml", e.id);
        if !is_slug(&e.id) {
            err.push(format!("{at}: file name must be lowercase-kebab-case"));
        }
        if !quads.contains(e.quadrant.as_str()) {
            err.push(format!("{at}: unknown quadrant `{}`", e.quadrant));
        }
        if e.name.trim().is_empty() {
            err.push(format!("{at}: name is empty"));
        }
        // One entry, one name: the scope shows the name, on two lines when it is long, or a label
        // made of the name's first words. A reader then finds the label's words at the start of
        // the name in every list.
        let fits_two_lines = || crate::layout::wrap2(&e.name).is_some_and(|ls| ls.iter().all(|l| l.chars().count() <= LABEL_MAX));
        match &e.label {
            Some(l) if l.trim().is_empty() || l.chars().count() > LABEL_MAX || l.chars().count() >= e.name.chars().count() => {
                err.push(format!("{at}: label must be 1-{LABEL_MAX} characters and shorter than the name, is `{l}`"));
            }
            Some(l) if !e.name.strip_prefix(l.as_str()).is_some_and(|rest| rest.starts_with([' ', ',', '(', ':'])) => {
                err.push(format!("{at}: label `{l}` must be the first words of the name `{}`", e.name));
            }
            None if e.name.chars().count() > LABEL_MAX && !fits_two_lines() => {
                warn.push(format!("{at}: name is longer than {LABEL_MAX} characters and does not split into two lines of {LABEL_MAX}; add a short `label` for the scope"));
            }
            _ => {}
        }
        let n = e.summary.chars().count();
        if n == 0 || n > 280 {
            err.push(format!("{at}: summary must be 1-280 characters, is {n}"));
        }
        if let Some(u) = &e.url
            && !is_url(u)
        {
            err.push(format!("{at}: url must be https"));
        }
        for ev in &e.evidence {
            if !is_url(&ev.url) {
                err.push(format!("{at}: evidence url `{}` must be https", ev.url));
            }
            if ev.claim.trim().is_empty() {
                err.push(format!("{at}: evidence for {} has no claim", ev.url));
            }
            if !ev.checked_on.valid() {
                err.push(format!("{at}: evidence checked_on is not a real date"));
            }
        }
        entries.insert(e.id.as_str(), e);
    }

    // Radars: opinions. Every change is dated to an edition and says why.
    let mut radar_ids = HashSet::new();
    let mut referenced = HashSet::new();
    for r in &d.radars {
        let at = format!("radars/{}.toml", r.id);
        radar_ids.insert(r.id.as_str());
        if !is_slug(&r.id) {
            err.push(format!("{at}: file name must be lowercase-kebab-case"));
        }
        if r.editions.is_empty() {
            err.push(format!("{at}: needs at least one [[edition]]"));
        }
        for w in r.editions.windows(2) {
            if w[0].date >= w[1].date {
                err.push(format!("{at}: editions must be in strictly ascending date order ({} then {})", w[0].date, w[1].date));
            }
        }
        for ed in &r.editions {
            if !ed.date.valid() {
                err.push(format!("{at}: edition date {} is not a real date", ed.date));
            }
            if ed.title.trim().is_empty() {
                err.push(format!("{at}: edition {} has no title", ed.date));
            }
        }
        let ed_dates: HashSet<_> = r.editions.iter().map(|e| e.date).collect();
        let mut in_radar = HashSet::new();
        for b in &r.blips {
            let bat = format!("{at}: blip `{}`", b.entry);
            let Some(e) = entries.get(b.entry.as_str()) else {
                err.push(format!("{bat}: no entries/{}.toml", b.entry));
                continue;
            };
            referenced.insert(b.entry.as_str());
            if !in_radar.insert(b.entry.as_str()) {
                err.push(format!("{bat}: listed twice"));
            }
            if b.history.is_empty() {
                err.push(format!("{bat}: needs at least one [[blip.history]] item"));
                continue;
            }
            if b.history[0].ring == RETIRED {
                err.push(format!("{bat}: history cannot start with `retired`"));
            }
            for w in b.history.windows(2) {
                if w[0].date >= w[1].date {
                    err.push(format!("{bat}: history dates must be strictly ascending ({} then {})", w[0].date, w[1].date));
                }
                if w[0].ring == w[1].ring {
                    err.push(format!("{bat}: history on {} repeats ring `{}`; record only changes", w[1].date, w[1].ring));
                }
            }
            for m in &b.history {
                if !rings.contains(m.ring.as_str()) && m.ring != RETIRED {
                    err.push(format!("{bat}: unknown ring `{}` on {}", m.ring, m.date));
                }
                if !ed_dates.contains(&m.date) {
                    err.push(format!("{bat}: change on {} has no matching [[edition]]", m.date));
                }
                if m.note.trim().is_empty() {
                    err.push(format!("{bat}: change on {} has no note; every move records why", m.date));
                }
            }
            let n = b.rationale.chars().count();
            if n > RATIONALE_MAX {
                warn.push(format!("{bat}: rationale is {n} characters, over {RATIONALE_MAX}; articles stay short"));
            }
            let retired = b.history.last().is_some_and(|m| m.ring == RETIRED);
            if !retired && b.rationale.trim().is_empty() {
                err.push(format!("{bat}: rationale is empty"));
            }
            // A practice from the owner's own work can have no outside source; say so, but don't block the build.
            if b.status == Status::Confirmed && r.kind == Kind::Curated && e.evidence.is_empty() {
                warn.push(format!("{bat}: confirmed on a curated radar, but entries/{}.toml has no evidence", b.entry));
            }
        }
        for rj in &r.rejected {
            if rj.reason.trim().is_empty() || rj.name.trim().is_empty() {
                err.push(format!("{at}: rejected item `{}` needs a name and a reason", rj.name));
            }
            if rj.url.as_deref().is_some_and(|u| !is_url(u)) {
                err.push(format!("{at}: rejected item `{}` url must be https", rj.name));
            }
        }
        let drafts = r.blips.iter().filter(|b| b.status == Status::Draft).count();
        if drafts > 0 && r.kind == Kind::Curated {
            warn.push(format!("{at}: {drafts} of {} blips have status = \"draft\"; the owner confirms them before an edition ships", r.blips.len()));
        }
    }
    for (file, field, text) in public_texts(d) {
        for what in leak(text, deny) {
            err.push(format!("{file}: {field} contains {what}; internal material stays out of public text"));
        }
    }
    if !radar_ids.contains(s.main_radar.as_str()) {
        err.push(format!("site.toml: main_radar `{}` has no radars/{}.toml", s.main_radar, s.main_radar));
    } else if d.radars.iter().any(|r| r.id == s.main_radar && r.kind != Kind::Curated) {
        err.push("site.toml: main_radar must have kind = \"curated\"".into());
    }
    let orphans: Vec<_> = d.entries.iter().filter(|e| !referenced.contains(e.id.as_str())).map(|e| e.id.as_str()).collect();
    if !orphans.is_empty() {
        warn.push(format!("entries on no radar: {}", orphans.join(", ")));
    }
    Report { errors: err, warnings: warn }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SITE: &str = r#"
title = "T"
tagline = "t"
description = "d"
base_url = "https://example.org"
main_radar = "main"
[[quadrant]]
id = "a"
name = "A"
position = "top-left"
description = ""
[[quadrant]]
id = "b"
name = "B"
position = "top-right"
description = ""
[[quadrant]]
id = "c"
name = "C"
position = "bottom-left"
description = ""
[[quadrant]]
id = "d"
name = "D"
position = "bottom-right"
description = ""
[[ring]]
id = "adopt"
name = "Adopt"
description = ""
[[ring]]
id = "trial"
name = "Trial"
description = ""
[[ring]]
id = "assess"
name = "Assess"
description = ""
[[ring]]
id = "hold"
name = "Hold"
description = ""
"#;

    const ENTRY: &str = "name = \"Thing\"\nquadrant = \"a\"\nsummary = \"A thing.\"\n";

    const RADAR: &str = r#"
title = "Main"
owner = "Me"
kind = "curated"
description = ""
[[edition]]
date = 2026-09-21
title = "One"
note = ""
[[edition]]
date = 2026-09-28
title = "Two"
note = ""
[[blip]]
entry = "thing"
status = "draft"
rationale = "Because."
[[blip.history]]
date = 2026-09-21
ring = "trial"
note = "Added."
[[blip.history]]
date = 2026-09-28
ring = "adopt"
note = "Moved in."
"#;

    fn run(site: &str, entry: &str, radar: &str) -> Result<Report, Vec<String>> {
        let d = Data::parse(site, &[("thing".into(), entry.into())], &[("main".into(), radar.into())])?;
        Ok(validate(&d))
    }

    fn errors(radar: &str) -> Vec<String> {
        run(SITE, ENTRY, radar).unwrap().errors
    }

    #[test]
    fn valid_fixture_passes() {
        assert_eq!(errors(RADAR), Vec::<String>::new());
    }

    #[test]
    fn repo_data_is_valid() {
        let d = Data::load(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/data"))).expect("data parses");
        assert_eq!(validate_with(&d, &load_denylist()).errors, Vec::<String>::new());
    }

    #[test]
    fn leak_check_builtin_patterns() {
        for bad in ["write to kk@example.org", "see /Users/kk/notes", "in ~/github/x", "the .env file", "on localhost:8000", "at 127.0.0.1"] {
            let e = errors(&RADAR.replace("rationale = \"Because.\"", &format!("rationale = \"{bad}\"")));
            assert!(e.iter().any(|x| x.contains("blip `thing` rationale contains")), "{bad}: {e:?}");
        }
        let e = errors(&RADAR.replace("title = \"Two\"", "title = \"Two\"\nbody = \"Para one.\\n\\nMail me@host.io.\""));
        assert!(e.iter().any(|x| x.contains("edition 2026-09-28 body contains an email address")), "{e:?}");
        for ok in ["follow @kgmkauf on X", "the environment matters", "dotenv files", "an .envoy proxy", "users of Claude"] {
            assert!(leak(ok, &[]).is_empty(), "{ok}: {:?}", leak(ok, &[]));
        }
    }

    #[test]
    fn leak_check_denylist_whole_words() {
        let deny = parse_denylist("# private\n\nAcme Health  # a client\nzorg\n");
        assert_eq!(deny, vec!["Acme Health".to_string(), "zorg".to_string()]);
        assert!(!leak("We built it for ACME health, fast.", &deny).is_empty());
        assert!(!leak("Zorg.", &deny).is_empty());
        assert!(!leak("Built for Acme\n   health.", &deny).is_empty());
        assert_eq!(leak("acme health", &["Acme   Health".to_string()]).len(), 1);
        assert!(leak("zorgen and bezorg are other words; Acme Healthy too", &deny).is_empty());
        let d = Data::parse(SITE, &[("thing".into(), ENTRY.replace("A thing.", "Made for zorg.").into())], &[("main".into(), RADAR.into())]).unwrap();
        let e = validate_with(&d, &deny).errors;
        assert!(e.iter().any(|x| x.contains("entries/thing.toml: summary contains the private term `zorg`")), "{e:?}");
        assert!(validate(&d).errors.is_empty());
    }

    #[test]
    fn ship_fails_on_draft() {
        let d = Data::parse(SITE, &[("thing".into(), ENTRY.into())], &[("main".into(), RADAR.into())]).unwrap();
        let e = ship_errors(&d);
        assert!(e.len() == 1 && e[0].contains("blip `thing` has status = \"draft\""), "{e:?}");
        let d = Data::parse(SITE, &[("thing".into(), ENTRY.into())], &[("main".into(), RADAR.replace("\"draft\"", "\"confirmed\""))]).unwrap();
        assert!(ship_errors(&d).is_empty());
    }

    #[test]
    fn long_rationale_warns() {
        let r = run(SITE, ENTRY, &RADAR.replace("Because.", &"word ".repeat(200))).unwrap();
        assert!(r.warnings.iter().any(|x| x.contains("articles stay short")), "{:?}", r.warnings);
    }

    #[test]
    fn unknown_field_fails_parse() {
        assert!(run(SITE, "name = \"X\"\nquadrant = \"a\"\nsummary = \"s\"\nsumary = \"typo\"\n", RADAR).is_err());
    }

    #[test]
    fn dangling_entry_reference() {
        let e = errors(&RADAR.replace("entry = \"thing\"", "entry = \"missing\""));
        assert!(e.iter().any(|x| x.contains("no entries/missing.toml")), "{e:?}");
    }

    #[test]
    fn unknown_ring_and_undated_move() {
        let e = errors(&RADAR.replace("ring = \"adopt\"", "ring = \"adoptt\"").replace("date = 2026-09-28\nring", "date = 2026-09-30\nring"));
        assert!(e.iter().any(|x| x.contains("unknown ring `adoptt`")), "{e:?}");
        assert!(e.iter().any(|x| x.contains("no matching [[edition]]")), "{e:?}");
    }

    #[test]
    fn move_without_reason_or_change() {
        let e = errors(&RADAR.replace("note = \"Moved in.\"", "note = \"\"").replace("ring = \"adopt\"", "ring = \"trial\""));
        assert!(e.iter().any(|x| x.contains("no note")), "{e:?}");
        assert!(e.iter().any(|x| x.contains("repeats ring")), "{e:?}");
    }

    #[test]
    fn confirmed_curated_without_evidence_warns() {
        let r = run(SITE, ENTRY, &RADAR.replace("status = \"draft\"", "status = \"confirmed\"")).unwrap();
        assert_eq!(r.errors, Vec::<String>::new());
        assert!(r.warnings.iter().any(|x| x.contains("has no evidence")), "{:?}", r.warnings);
    }

    #[test]
    fn scope_label_length() {
        let entry = |name: &str, label: &str| {
            let label = if label.is_empty() { String::new() } else { format!("label = \"{label}\"\n") };
            run(SITE, &format!("name = \"{name}\"\n{label}quadrant = \"a\"\nsummary = \"s\"\n"), RADAR).unwrap()
        };
        // Too long for two lines of the scope, and no label.
        let r = entry("A very long name for a thing that goes on and on", "");
        assert!(r.warnings.iter().any(|x| x.contains("add a short `label`")), "{:?}", r.warnings);
        // Two lines of at most 18 characters: the scope shows the whole name.
        let r = entry("A rather long name for a thing", "");
        assert!(r.errors.is_empty() && r.warnings.iter().all(|x| !x.contains("label")), "{:?} {:?}", r.errors, r.warnings);
        let r = entry("A rather long name for a thing", "Still a rather long name");
        assert!(r.errors.iter().any(|x| x.contains("label must be")), "{:?}", r.errors);
        // A label is the name's first words, so the scope and the lists share one name.
        let r = entry("A rather long name for a thing", "Long thing");
        assert!(r.errors.iter().any(|x| x.contains("first words")), "{:?}", r.errors);
        let r = entry("A rather long name for a thing", "A rather long");
        assert!(r.errors.is_empty() && r.warnings.iter().all(|x| !x.contains("label")), "{:?} {:?}", r.errors, r.warnings);
    }

    #[test]
    fn bad_quadrant_and_url() {
        let r = run(SITE, "name = \"X\"\nquadrant = \"zz\"\nurl = \"http://x\"\nsummary = \"s\"\n", RADAR).unwrap();
        assert!(r.errors.iter().any(|x| x.contains("unknown quadrant")));
        assert!(r.errors.iter().any(|x| x.contains("url must be https")));
    }
}
