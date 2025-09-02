use core::ops::Deref;

// Functions for accessing phys frames
use x86_64::{PhysAddr, VirtAddr};

use crate::*;

pub trait TranslateToVirt {
    fn to_virt(self, paging: &PagingConfig) -> VirtAddr;
}

impl TranslateToVirt for PhysAddr {
    fn to_virt(self, paging: &PagingConfig) -> VirtAddr {
        VirtAddr::new(self.as_u64() + paging.offset.deref())
    }
}
