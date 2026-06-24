//! Shared types for the global search index.

/// A deployed contract registered in the search index.
#[derive(Debug, Clone, PartialEq)]
pub struct ContractEntry {
    /// On-chain contract address (Stellar/Soroban contract ID).
    pub contract_id: String,
    /// Human-readable contract name (e.g. "vesting_contracts").
    pub name: String,
    /// All public function IDs exported by this contract.
    pub function_ids: Vec<String>,
}

impl ContractEntry {
    pub fn new(contract_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            contract_id: contract_id.into(),
            name: name.into(),
            function_ids: Vec::new(),
        }
    }

    pub fn with_functions(mut self, fns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.function_ids = fns.into_iter().map(Into::into).collect();
        self
    }
}

/// A single search hit returned by [`crate::fuzzy_search`].
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    /// The matching contract entry.
    pub entry: ContractEntry,
    /// Which field matched: `"name"`, `"contract_id"`, or a function ID.
    pub matched_field: String,
    /// Fuzzy score in `[0, 1]`; 1.0 = exact match.
    pub score: f32,
}
