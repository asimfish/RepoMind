// Indexer service - wraps GitNexus/custom indexing logic
// TODO: Integrate with LadybugDB and Tree-sitter

pub struct IndexerConfig {
    pub repo_path: String,
    pub db_path: String,
}
