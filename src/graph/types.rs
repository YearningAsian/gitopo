use git2::Oid;

#[derive(Debug, Clone)]
pub struct GraphRow {
    pub oid: Oid,
    /// Column this commit sits in
    pub col: usize,
    /// Total active columns in this row
    pub num_cols: usize,
    /// Connector lines below this row (indexed by column)
    pub connectors: Vec<Connector>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Connector {
    Vertical,
    Empty,
}
