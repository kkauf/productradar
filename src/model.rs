//! Raw data files, deserialized as written. Unknown fields are errors, so typos fail the build.

use crate::date::Date;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Site {
    pub title: String,
    pub tagline: String,
    pub description: String,
    pub base_url: String,
    pub main_radar: String,
    #[serde(rename = "quadrant")]
    pub quadrants: Vec<Quadrant>,
    #[serde(rename = "ring")]
    pub rings: Vec<Ring>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Quadrant {
    pub id: String,
    pub name: String,
    pub position: Corner,
    pub description: String,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Ring {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    #[serde(skip)]
    pub id: String,
    pub name: String,
    /// A short name for the scope only (at most `LABEL_MAX` characters). The full name shows
    /// everywhere else, including the scope's hover tag.
    #[serde(default)]
    pub label: Option<String>,
    pub quadrant: String,
    #[serde(default)]
    pub maker: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    pub summary: String,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
}

/// Longest name the scope shows; a longer name needs a `label`.
pub const LABEL_MAX: usize = 18;

impl Entry {
    /// The name the scope shows: the short label when there is one.
    pub fn scope_name(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.name)
    }
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub url: String,
    pub checked_on: Date,
    pub claim: String,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Radar {
    #[serde(skip)]
    pub id: String,
    pub title: String,
    pub owner: String,
    pub kind: Kind,
    #[serde(default)]
    pub example: bool,
    pub description: String,
    #[serde(rename = "edition")]
    pub editions: Vec<Edition>,
    #[serde(rename = "blip", default)]
    pub blips: Vec<Blip>,
    #[serde(rename = "rejected", default)]
    pub rejected: Vec<Rejected>,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Curated,
    Community,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Edition {
    pub date: Date,
    pub title: String,
    pub note: String,
    /// A short article: paragraphs separated by blank lines. Shows on the edition's own page,
    /// in the feed, in llms.txt and in radar.json; the list on /moves/ shows only the note.
    #[serde(default)]
    pub body: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Blip {
    pub entry: String,
    pub status: Status,
    /// One to four paragraphs separated by blank lines. One-line places use `first_para`.
    pub rationale: String,
    pub history: Vec<Move>,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Draft,
    Confirmed,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Move {
    pub date: Date,
    /// A ring id, or "retired" when the entry leaves the radar.
    pub ring: String,
    pub note: String,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Rejected {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    pub date: Date,
    pub reason: String,
}

pub struct Data {
    pub site: Site,
    pub entries: Vec<Entry>,
    pub radars: Vec<Radar>,
}

pub const RETIRED: &str = "retired";

impl Data {
    /// Parse from in-memory sources: (id, toml) pairs. Collects every parse error.
    pub fn parse(site: &str, entries: &[(String, String)], radars: &[(String, String)]) -> Result<Data, Vec<String>> {
        let mut errs = Vec::new();
        let site: Option<Site> = toml::from_str(site).map_err(|e| errs.push(format!("site.toml: {e}"))).ok();
        let mut es = Vec::new();
        for (id, src) in entries {
            match toml::from_str::<Entry>(src) {
                Ok(mut e) => {
                    e.id = id.clone();
                    es.push(e)
                }
                Err(e) => errs.push(format!("entries/{id}.toml: {e}")),
            }
        }
        let mut rs = Vec::new();
        for (id, src) in radars {
            match toml::from_str::<Radar>(src) {
                Ok(mut r) => {
                    r.id = id.clone();
                    rs.push(r)
                }
                Err(e) => errs.push(format!("radars/{id}.toml: {e}")),
            }
        }
        match site {
            Some(site) if errs.is_empty() => {
                es.sort_by(|a, b| a.id.cmp(&b.id));
                let main = site.main_radar.clone();
                rs.sort_by(|a, b| (a.id != main, &a.id).cmp(&(b.id != main, &b.id)));
                Ok(Data { site, entries: es, radars: rs })
            }
            _ => Err(errs),
        }
    }

    pub fn load(dir: &Path) -> Result<Data, Vec<String>> {
        let read = |p: &Path| std::fs::read_to_string(p).map_err(|e| vec![format!("{}: {e}", p.display())]);
        let site = read(&dir.join("site.toml"))?;
        let entries = read_dir_toml(&dir.join("entries"))?;
        let radars = read_dir_toml(&dir.join("radars"))?;
        Data::parse(&site, &entries, &radars)
    }
}

fn read_dir_toml(dir: &Path) -> Result<Vec<(String, String)>, Vec<String>> {
    let rd = std::fs::read_dir(dir).map_err(|e| vec![format!("{}: {e}", dir.display())])?;
    let mut out = Vec::new();
    for ent in rd.flatten() {
        let p = ent.path();
        if p.extension().is_some_and(|x| x == "toml") {
            let id = p.file_stem().unwrap().to_string_lossy().into_owned();
            let src = std::fs::read_to_string(&p).map_err(|e| vec![format!("{}: {e}", p.display())])?;
            out.push((id, src));
        }
    }
    out.sort();
    Ok(out)
}
