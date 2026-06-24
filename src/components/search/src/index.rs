//! In-memory index of deployed contracts, optimized for local-state lookup.

use crate::types::ContractEntry;

/// In-memory index that holds all registered [`ContractEntry`] items.
///
/// Built once at startup (or on contract deployment events) and queried
/// repeatedly at zero I/O cost — satisfying the "performance optimized via
/// local state" requirement.
#[derive(Default)]
pub struct SearchIndex {
    entries: Vec<ContractEntry>,
}

impl SearchIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a contract in the index.
    pub fn insert(&mut self, entry: ContractEntry) {
        // Replace existing entry with the same contract_id to keep index fresh.
        if let Some(pos) = self
            .entries
            .iter()
            .position(|e| e.contract_id == entry.contract_id)
        {
            self.entries[pos] = entry;
        } else {
            self.entries.push(entry);
        }
    }

    /// Remove a contract from the index by its contract_id.
    pub fn remove(&mut self, contract_id: &str) {
        self.entries.retain(|e| e.contract_id != contract_id);
    }

    /// Immutable view of all indexed entries.
    pub fn entries(&self) -> &[ContractEntry] {
        &self.entries
    }

    /// Number of contracts currently indexed.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
