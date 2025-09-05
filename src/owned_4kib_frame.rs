use core::ops::Deref;

use x86_64::structures::paging::PhysFrame;

/// A wrapper around [`PhysFrame`] that guarantees that it is "owned" by whatever owns it.
#[derive(Debug)]
pub struct Owned4KibFrame(pub(crate) PhysFrame);

impl Owned4KibFrame {
    /// # Safety
    /// - The phys frame must be valid physical memory
    /// - The memory cannot be used or referenced by anything else
    pub unsafe fn new(frame: PhysFrame) -> Self {
        Self(frame)
    }
}

impl Deref for Owned4KibFrame {
    type Target = PhysFrame;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Owned4KibFrame> for PhysFrame {
    fn from(value: Owned4KibFrame) -> Self {
        value.0
    }
}
