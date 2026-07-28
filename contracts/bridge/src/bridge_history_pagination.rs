// SPDX-License-Identifier: MIT
pub struct PaginatedBridgeHistory {
    pub max_entries_per_account: usize,
}

impl Default for PaginatedBridgeHistory {
    fn default() -> Self {
        Self::new(100)
    }
}

impl PaginatedBridgeHistory {
    pub fn new(max_entries_per_account: usize) -> Self {
        Self {
            max_entries_per_account,
        }
    }

    pub fn paginate<T: Clone>(&self, history: &[T], page: usize, page_size: usize) -> Vec<T> {
        let start = page.saturating_mul(page_size);
        if start >= history.len() {
            return Vec::new();
        }
        let end = (start + page_size).min(history.len());
        history[start..end].to_vec()
    }
}
