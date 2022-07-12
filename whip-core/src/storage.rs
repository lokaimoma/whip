use futures::io::Cursor;
use tokio::fs::File;

#[derive(Debug)]
pub enum Storage {
    InMemory(MemoryStorage),
    File(FileStorage),
}

#[derive(Debug)]
pub struct MemoryStorage {
    pub cursor: Cursor<Vec<u8>>,
}

impl MemoryStorage {
    pub fn new(size: u64) -> Self {
        MemoryStorage {
            cursor: Cursor::new(Vec::with_capacity(
                size.try_into().unwrap_or(usize::max_value()),
            )),
        }
    }
}

#[derive(Debug)]
pub struct FileStorage {
    pub file: File,
}
