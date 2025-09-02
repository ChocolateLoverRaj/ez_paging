use core::ops::Deref;

#[derive(Debug, Clone, Copy)]
pub struct VirtualOffset(u64);

impl VirtualOffset {
    /// # Safety
    /// All physical memory should be mapped starting at this virtual offset
    pub unsafe fn new(virtual_offset: u64) -> Self {
        Self(virtual_offset)
    }
}

impl Deref for VirtualOffset {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
