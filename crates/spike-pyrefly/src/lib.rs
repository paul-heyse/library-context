//! S1 build spike: every family must co-resolve in one graph.

pub fn types_link() -> usize {
    std::mem::size_of::<pyrefly::state::require::Require>()
        + std::mem::size_of::<ruff_text_size::TextRange>()
        + std::mem::size_of::<deltalake::DeltaTable>()
        + std::mem::size_of::<datafusion::prelude::SessionContext>()
}
