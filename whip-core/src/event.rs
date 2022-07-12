use crate::storage::Storage;

#[derive(Debug)]
pub enum Event {
    ProgressChanged(u64),
    Complete(CompleteStats),
}

#[derive(Debug)]
pub struct CompleteStats {
    pub storage: Storage,
    pub part_id: u8,
}
