//! Pre-sorted, pre-lowercased board search index grouped by vendor with query & filter caching.

use ravom_core::registry::BoardRecord;

/// Ecosystem filter chip for narrowing the board catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardFilter {
    All,
    ProbeRs,
    Esp32,
    Stm32,
    Rp2040,
    Nrf,
}

impl BoardFilter {
    /// All filter variants in display order.
    pub const ALL: &'static [BoardFilter] = &[
        BoardFilter::All,
        BoardFilter::ProbeRs,
        BoardFilter::Esp32,
        BoardFilter::Stm32,
        BoardFilter::Rp2040,
        BoardFilter::Nrf,
    ];

    /// Short display label for filter chips.
    pub fn label(self) -> &'static str {
        match self {
            BoardFilter::All => "All",
            BoardFilter::ProbeRs => "probe-rs",
            BoardFilter::Esp32 => "ESP32",
            BoardFilter::Stm32 => "STM32",
            BoardFilter::Rp2040 => "RP2040/2350",
            BoardFilter::Nrf => "nRF",
        }
    }

    /// Return whether `board` belongs to this ecosystem filter.
    pub fn matches(self, board: &BoardRecord) -> bool {
        match self {
            BoardFilter::All => true,
            BoardFilter::ProbeRs => board.probe_rs_chip.is_some(),
            BoardFilter::Esp32 => {
                board.platform.starts_with("espressif")
                    || board.mcu.to_ascii_uppercase().starts_with("ESP32")
            }
            BoardFilter::Stm32 => {
                board.platform.contains("stm32")
                    || board.mcu.to_ascii_uppercase().starts_with("STM32")
            }
            BoardFilter::Rp2040 => {
                board.platform.contains("raspberrypi")
                    || board.mcu.to_ascii_uppercase().starts_with("RP2")
            }
            BoardFilter::Nrf => {
                board.platform.contains("nordic")
                    || board.mcu.to_ascii_uppercase().starts_with("NRF")
            }
        }
    }
}

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

/// Indexed, pre-sorted board catalog with cached vendor groupings.
pub struct BoardIndex {
    boards: Vec<BoardRecord>,
    haystacks: Vec<String>,
    cached_query: String,
    cached_filter: BoardFilter,
    cached_terms: Vec<String>,
    cached_groups: Vec<VendorGroup>,
    cached_total: usize,
}

impl BoardIndex {
    /// Build a new sorted board index from `boards`.
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
            cached_filter: BoardFilter::All,
            cached_terms: Vec::new(),
            cached_groups: Vec::new(),
            cached_total: 0,
        };
        idx.ensure_query("", BoardFilter::All);
        idx
    }

    /// Total number of boards loaded in the catalog.
    pub fn len(&self) -> usize {
        self.boards.len()
    }

    /// Fetch board record by flat index.
    pub fn get(&self, idx: usize) -> &BoardRecord {
        &self.boards[idx]
    }

    /// Look up a board by its qualified `<platform>/<id>` identifier.
    pub fn find_by_qualified(&self, qualified: &str) -> Option<&BoardRecord> {
        self.boards.iter().find(|b| b.qualified_id() == qualified)
    }

    /// Recompute cached vendor groups only when `query` or `filter` changes.
    pub fn ensure_query(&mut self, query: &str, filter: BoardFilter) {
        if self.cached_query == query && self.cached_filter == filter {
            return;
        }
        self.cached_query.clear();
        self.cached_query.push_str(query);
        self.cached_filter = filter;
        self.cached_terms = query.split_whitespace().map(str::to_lowercase).collect();
        self.cached_groups.clear();
        self.cached_total = 0;

        for (i, (board, hay)) in self.boards.iter().zip(&self.haystacks).enumerate() {
            if !filter.matches(board) {
                continue;
            }
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

    /// Lowercased whitespace-separated query terms currently cached.
    pub fn cached_terms(&self) -> &[String] {
        &self.cached_terms
    }

    /// Cached vendor groups matching the current query and filter.
    pub fn cached_groups(&self) -> &[VendorGroup] {
        &self.cached_groups
    }

    /// Total matching board count for the current query and filter.
    pub fn cached_total(&self) -> usize {
        self.cached_total
    }

    #[cfg(test)]
    pub fn filter<'a>(&'a self, query: &str) -> impl Iterator<Item = &'a BoardRecord> + 'a {
        let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        self.boards
            .iter()
            .zip(&self.haystacks)
            .filter(move |(_, hay)| terms.iter().all(|t| hay.contains(t.as_str())))
            .map(|(b, _)| b)
    }

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

        index.ensure_query("adafruit", BoardFilter::All);
        assert!(index.cached_total() > 0);
        assert!(index
            .cached_groups()
            .iter()
            .any(|g| g.vendor.eq_ignore_ascii_case("Adafruit")));
    }

    #[test]
    fn ecosystem_filters_and_qualified_lookup_work() {
        let mut index = BoardIndex::new(BoardRegistry::embedded().boards().cloned().collect());
        let devkit = index
            .find_by_qualified("espressif32/esp32-c3-devkitm-1")
            .expect("esp32-c3-devkitm-1 should exist");
        assert_eq!(devkit.mcu, "ESP32C3");

        index.ensure_query("esp32c3", BoardFilter::Esp32);
        assert!(index.cached_total() > 0);

        index.ensure_query("", BoardFilter::ProbeRs);
        assert!(index.cached_total() > 1000);
    }
}
