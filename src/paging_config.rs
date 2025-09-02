use crate::*;

#[derive(Debug, Clone, Copy)]
pub struct PagingConfig {
    pub(crate) offset: VirtualOffset,
    pub(crate) pat: ManagedPat,
}

impl PagingConfig {
    pub fn new(pat: ManagedPat, offset: VirtualOffset) -> Self {
        Self { pat, offset }
    }
}
