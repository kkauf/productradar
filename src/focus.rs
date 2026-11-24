//! Focus editor page: one checkbox line per blip on the main radar's latest edition,
//! grouped by quadrant, with a ring picker. Printed to stdout only, never written to a file,
//! so a regeneration can never overwrite the owner's comments in the page.

use crate::view::Model;

pub fn page(m: &Model, title: &str) -> String {
    let main = m.main();
    let latest = main.latest();
    let rings: Vec<String> = m.site.rings.iter().map(|r| format!("#{}", r.id)).collect();
    // The Focus editor's status sections (since 2026-10-12): the entries wait under Backlog, which the queue never starts.
    let mut s = format!("# {title}\n\n## Outcome\n\n## In progress\n\n## Up next\n\n## Waiting\n\n## Backlog\nPicker: ring {}\n", rings.join(" "));
    for (q, quad) in m.site.quadrants.iter().enumerate() {
        s.push_str(&format!("\n### {}\n\n", quad.name));
        // `blips` is already sorted by lowercase name; a stable sort by ring keeps that order.
        let mut bs: Vec<(usize, &crate::view::BlipView)> = main.on_radar(latest).filter(|(_, b)| b.q == q).map(|(_, b)| (b.rings[latest].unwrap(), b)).collect();
        bs.sort_by_key(|(r, _)| *r);
        for (r, b) in bs {
            // A box line holds one line: the rationale's first paragraph. The entry page has the rest.
            let rationale = crate::html::first_para(&b.b.rationale);
            s.push_str(&format!("- [ ] {} — {} #{} <!--e:{}-->\n", b.e.name, rationale, m.site.rings[r].id, b.e.id));
        }
    }
    s.push_str("\n## Done\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Data;

    #[test]
    fn focus_page_from_real_data() {
        let data = Data::load(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/data"))).expect("data loads");
        let m = Model::new(&data);
        let page = page(&m, "Product Radar");
        let lines: Vec<&str> = page.lines().collect();

        assert_eq!(lines[0], "# Product Radar");
        let heads: Vec<&str> = lines.iter().copied().filter(|l| l.starts_with("## ")).collect();
        assert_eq!(heads, ["## Outcome", "## In progress", "## Up next", "## Waiting", "## Backlog", "## Done"]);
        let backlog = lines.iter().position(|l| *l == "## Backlog").unwrap();
        let first = lines[backlog + 1..].iter().find(|l| !l.is_empty()).unwrap();
        assert_eq!(*first, "Picker: ring #adopt #trial #assess #hold");

        // Quadrant headings in site.toml order.
        let quads: Vec<&str> = lines.iter().filter_map(|l| l.strip_prefix("### ")).collect();
        let want: Vec<&str> = m.site.quadrants.iter().map(|q| q.name.as_str()).collect();
        assert_eq!(quads, want);

        // Every box line matches `- [ ] <name> — <rationale> #<ring> <!--e:<id>-->`; record its quadrant.
        let ring_ids: Vec<&str> = m.site.rings.iter().map(|r| r.id.as_str()).collect();
        let mut quad = None;
        let mut seen: Vec<(String, String, String)> = Vec::new(); // (id, quadrant, ring)
        for l in &lines {
            if let Some(q) = l.strip_prefix("### ") {
                quad = Some(q.to_string());
                continue;
            }
            if !l.starts_with("- [ ]") {
                continue;
            }
            let rest = l.strip_prefix("- [ ] ").expect("box prefix");
            let (name, rest) = rest.split_once(" — ").expect("em dash with spaces");
            assert!(!name.is_empty() && !name.contains('\n'));
            let rest = rest.strip_suffix("-->").expect("hidden id suffix");
            let (rest, id) = rest.rsplit_once(" <!--e:").expect("hidden id");
            let (rationale, ring) = rest.rsplit_once(" #").expect("ring tag");
            assert!(!rationale.trim().is_empty(), "{l}");
            assert!(ring_ids.contains(&ring), "{l}");
            assert!(!id.is_empty() && !id.contains(' '), "{l}");
            seen.push((id.to_string(), quad.clone().expect("box under a quadrant"), ring.to_string()));
        }

        let main = m.main();
        let latest = main.latest();
        let expected: Vec<_> = main.on_radar(latest).collect();
        assert_eq!(seen.len(), expected.len(), "one line per blip");
        for (_, b) in expected {
            let hits: Vec<_> = seen.iter().filter(|(id, _, _)| *id == b.e.id).collect();
            assert_eq!(hits.len(), 1, "{} appears exactly once", b.e.id);
            assert_eq!(hits[0].1, m.quad_name(b.q), "{} under its quadrant", b.e.id);
            assert_eq!(hits[0].2, m.site.rings[b.rings[latest].unwrap()].id, "{} ring tag", b.e.id);
        }
    }
}
