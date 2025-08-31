/// Represents a key in a DBC file
pub type Key = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringRef(pub u32);

impl StringRef {
    pub fn new(offset: u32) -> Self {
        Self(offset)
    }

    pub fn offset(&self) -> u32 {
        self.0
    }
}
