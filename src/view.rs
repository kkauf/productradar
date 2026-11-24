//! Derived views over validated data: ring timelines, per-edition states and moves.

use crate::date::Date;
use crate::model::*;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Absent,
    New,
    In,
    Out,
    Steady,
    Retired,
}

impl State {
    pub fn class(self) -> &'static str {
        match self {
            State::Absent => "s-absent",
            State::New => "s-new",
            State::In => "s-in",
            State::Out => "s-out",
            State::Steady => "s-steady",
            State::Retired => "s-retired",
        }
    }
    pub fn code(self) -> u8 {
        self as u8
    }
    /// Short glyph used in lists; shape carries the meaning alongside color.
    pub fn glyph(self) -> &'static str {
        match self {
            State::New => "◆",
            State::In => "▲",
            State::Out => "▼",
            State::Retired => "×",
            _ => "●",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            State::New => "New",
            State::In => "Moved in",
            State::Out => "Moved out",
            State::Retired => "Retired",
            State::Steady => "No change",
            State::Absent => "Not on the radar",
        }
    }
}

pub struct BlipView<'a> {
    pub b: &'a Blip,
    pub e: &'a Entry,
    pub q: usize,
    /// Ring index at each edition; None when not on the radar.
    pub rings: Vec<Option<usize>>,
}

impl BlipView<'_> {
    pub fn ring_now(&self) -> Option<usize> {
        *self.rings.last().unwrap()
    }
    pub fn state_at(&self, e: usize) -> State {
        let now = self.rings[e];
        let before = if e == 0 { None } else { self.rings[e - 1] };
        match (before, now) {
            (None, None) => State::Absent,
            (None, Some(_)) => State::New,
            (Some(_), None) => State::Retired,
            (Some(a), Some(b)) if b < a => State::In,
            (Some(a), Some(b)) if b > a => State::Out,
            _ => State::Steady,
        }
    }
    pub fn first_seen(&self) -> Date {
        self.b.history[0].date
    }
    /// Latest edition index at which this blip changed.
    pub fn last_change(&self) -> usize {
        (0..self.rings.len()).rev().find(|&e| !matches!(self.state_at(e), State::Steady | State::Absent)).unwrap_or(0)
    }
}

pub struct MoveView<'a> {
    pub blip: usize,
    pub state: State,
    pub from: Option<usize>,
    pub to: Option<usize>,
    pub note: &'a str,
}

pub struct RadarView<'a> {
    pub r: &'a Radar,
    pub dates: Vec<Date>,
    pub blips: Vec<BlipView<'a>>,
}

impl<'a> RadarView<'a> {
    pub fn latest(&self) -> usize {
        self.dates.len() - 1
    }
    pub fn edition(&self, e: usize) -> &'a Edition {
        &self.r.editions[e]
    }
    pub fn moves_at(&self, e: usize) -> Vec<MoveView<'a>> {
        let date = self.dates[e];
        let mut out: Vec<MoveView> = self
            .blips
            .iter()
            .enumerate()
            .filter_map(|(i, bv)| {
                let st = bv.state_at(e);
                if matches!(st, State::Absent | State::Steady) {
                    return None;
                }
                let note = bv.b.history.iter().find(|m| m.date == date).map(|m| m.note.as_str()).unwrap_or("");
                Some(MoveView { blip: i, state: st, from: if e == 0 { None } else { bv.rings[e - 1] }, to: bv.rings[e], note })
            })
            .collect();
        let rank = |s: State| match s {
            State::New => 0,
            State::In => 1,
            State::Out => 2,
            _ => 3,
        };
        out.sort_by(|a, b| (rank(a.state), &self.blips[a.blip].e.name).cmp(&(rank(b.state), &self.blips[b.blip].e.name)));
        out
    }
    pub fn on_radar(&self, e: usize) -> impl Iterator<Item = (usize, &BlipView<'a>)> {
        self.blips.iter().enumerate().filter(move |(_, b)| b.rings[e].is_some())
    }
    pub fn count(&self, e: usize, st: State) -> usize {
        self.blips.iter().filter(|b| b.state_at(e) == st).count()
    }
    pub fn find(&self, entry: &str) -> Option<&BlipView<'a>> {
        self.blips.iter().find(|b| b.e.id == entry)
    }
}

pub struct Model<'a> {
    pub site: &'a Site,
    pub entries: &'a [Entry],
    pub radars: Vec<RadarView<'a>>,
}

impl<'a> Model<'a> {
    pub fn new(d: &'a Data) -> Model<'a> {
        let ring_ix: HashMap<&str, usize> = d.site.rings.iter().enumerate().map(|(i, r)| (r.id.as_str(), i)).collect();
        let quad_ix: HashMap<&str, usize> = d.site.quadrants.iter().enumerate().map(|(i, q)| (q.id.as_str(), i)).collect();
        let entry: HashMap<&str, &Entry> = d.entries.iter().map(|e| (e.id.as_str(), e)).collect();
        let radars = d
            .radars
            .iter()
            .map(|r| {
                let dates: Vec<Date> = r.editions.iter().map(|e| e.date).collect();
                let mut blips: Vec<BlipView> = r
                    .blips
                    .iter()
                    .map(|b| {
                        let e = entry[b.entry.as_str()];
                        let mut rings = Vec::with_capacity(dates.len());
                        let mut cur = None;
                        let mut h = b.history.iter().peekable();
                        for d in &dates {
                            while let Some(m) = h.next_if(|m| m.date <= *d) {
                                cur = ring_ix.get(m.ring.as_str()).copied();
                            }
                            rings.push(cur);
                        }
                        BlipView { b, e, q: quad_ix[e.quadrant.as_str()], rings }
                    })
                    .collect();
                blips.sort_by_key(|b| b.e.name.to_lowercase());
                RadarView { r, dates, blips }
            })
            .collect();
        Model { site: &d.site, entries: &d.entries, radars }
    }
    pub fn main(&self) -> &RadarView<'a> {
        &self.radars[0]
    }
    /// What visitors call the curated radar ("Konstantin's radar"): its `title` in data/radars/.
    pub fn curated(&self) -> &str {
        &self.main().r.title
    }
    /// The curator's first name, for compact comparisons ("Konstantin: Adopt").
    pub fn curator(&self) -> &str {
        self.main().r.owner.split_whitespace().next().unwrap_or(&self.main().r.owner)
    }
    pub fn ring_name(&self, k: usize) -> &str {
        &self.site.rings[k].name
    }
    pub fn quad_name(&self, q: usize) -> &str {
        &self.site.quadrants[q].name
    }
}
