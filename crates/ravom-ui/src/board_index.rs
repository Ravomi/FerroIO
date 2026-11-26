//! Pre-sorted, pre-lowercased board search index grouped by vendor with query caching.

use ravom_core::registry::BoardRecord;

/// Display label for a board's vendor (falls back to platform when absent).
pub fn vendor_label(board: &BoardRecord) -> &str {
    board
        .vendor
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(board.platform.as_str())
}

/// Cached vendor group referencing indices into `BoardIndex::boards`.
pub struct VendorGroup {
    pub vendor: String,
    pub indices: Vec<usize>,
}

pub struct BoardIndex {
    boards: Vec<BoardRecord>,
    haystacks: Vec<String>,
    cached_query: String,
    cached_terms: Vec<String>,
    cached_groups: Vec<VendorGroup>,
    cached_total: usize,
}

impl BoardIndex {
    pub fn new(mut boards: Vec<BoardRecord>) -> Self {
        boards.sort_by(|a, b| {
            let va = vendor_label(a).to_lowercase();
            let vb = vendor_label(b).to_lowercase();
            va.cmp(&vb)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                .then_with(|| a.id.to_lowercase().cmp(&b.id.to_lowercase()))
        });
        let haystacks = boards
            .iter()
            .map(|b| {
                format!(
                    "{} {} {} {} {}",
                    vendor_label(b),
                    b.id,
                    b.platform,
                    b.mcu,
                    b.name
                )
                .to_lowercase()
            })
            .collect();
        let mut idx = Self {
            boards,
            haystacks,
            cached_query: String::from("\0"), // sentinel so initial "" builds cache
            cached_terms: Vec::new(),
            cached_groups: Vec::new(),
            cached_total: 0,
        };
        idx.ensure_query("");
        idx
    }

    pub fn len(&self) -> usize {
        self.boards.len()
    }

    pub fn get(&self, idx: usize) -> &BoardRecord {
        &self.boards[idx]
    }

    /// Recompute cached vendor groups only when `query` changes (zero per-frame allocations).
    pub fn ensure_query(&mut self, query: &str) {
        if self.cached_query == query {
            return;
        }
        self.cached_query.clear();
        self.cached_query.push_str(query);
        self.cached_terms = query.split_whitespace().map(str::to_lowercase).collect();
        self.cached_groups.clear();
        self.cached_total = 0;

        for (i, (board, hay)) in self.boards.iter().zip(&self.haystacks).enumerate() {
            if !self.cached_terms.iter().all(|t| hay.contains(t.as_str())) {
                continue;
            }
            self.cached_total += 1;
            let vendor = vendor_label(board);
            match self.cached_groups.last_mut() {
                Some(group) if group.vendor.eq_ignore_ascii_case(vendor) => {
                    group.indices.push(i);
                }
                _ => self.cached_groups.push(VendorGroup {
                    vendor: vendor.to_string(),
                    indices: vec![i],
                }),
            }
        }
    }

    pub fn cached_terms(&self) -> &[String] {
        &self.cached_terms
    }

    pub fn cached_groups(&self) -> &[VendorGroup] {
        &self.cached_groups
    }

    pub fn cached_total(&self) -> usize {
        self.cached_total
    }

    /// Boards whose vendor/id/platform/MCU/name contain every whitespace-separated term,
    /// in `(vendor, board name, board id)` alphabetical order.
    #[cfg(test)]
    pub fn filter<'a>(&'a self, query: &str) -> impl Iterator<Item = &'a BoardRecord> + 'a {
        let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        self.boards
            .iter()
            .zip(&self.haystacks)
            .filter(move |(_, hay)| terms.iter().all(|t| hay.contains(t.as_str())))
            .map(|(b, _)| b)
    }

    /// Filtered boards grouped by vendor (both vendors and boards within each vendor
    /// are sorted alphabetically, case-insensitively).
    #[cfg(test)]
    pub fn grouped_filter<'a>(&'a self, query: &str) -> Vec<(&'a str, Vec<&'a BoardRecord>)> {
        let mut groups: Vec<(&'a str, Vec<&'a BoardRecord>)> = Vec::new();
        for board in self.filter(query) {
            let vendor = vendor_label(board);
            match groups.last_mut() {
                Some((last_vendor, list)) if last_vendor.eq_ignore_ascii_case(vendor) => {
                    list.push(board);
                }
                _ => groups.push((vendor, vec![board])),
            }
        }
        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ravom_core::registry::BoardRegistry;

    #[test]
    fn filters_by_all_terms_case_insensitively() {
        let index = BoardIndex::new(BoardRegistry::embedded().boards().cloned().collect());
        assert!(index.len() > 2000);
        let hits: Vec<_> = index.filter("RP2040 pico").collect();
        assert!(hits.iter().any(|b| b.id == "pico"));
        assert!(hits.iter().all(|b| b.mcu.contains("RP2040")));
        assert_eq!(index.filter("").count(), index.len());
        assert_eq!(index.filter("zzz-no-such-board").count(), 0);
    }

    #[test]
    fn groups_by_vendor_and_sorts_alphabetically() {
        let mut index = BoardIndex::new(BoardRegistry::embedded().boards().cloned().collect());
        let groups = index.grouped_filter("");
        assert!(groups.len() > 50);

        for win in groups.windows(2) {
            assert!(
                win[0].0.to_lowercase() <= win[1].0.to_lowercase(),
                "vendors out of order: {} > {}",
                win[0].0,
                win[1].0
            );
        }

        for (vendor, boards) in &groups {
            for win in boards.windows(2) {
                let a = (win[0].name.to_lowercase(), win[0].id.to_lowercase());
                let b = (win[1].name.to_lowercase(), win[1].id.to_lowercase());
                assert!(
                    a <= b,
                    "boards under {vendor} out of order: {:?} > {:?}",
                    a,
                    b
                );
            }
        }

        index.ensure_query("adafruit");
        assert!(index.cached_total() > 0);
        assert!(index
            .cached_groups()
            .iter()
            .any(|g| g.vendor.eq_ignore_ascii_case("Adafruit")));
    }
}
