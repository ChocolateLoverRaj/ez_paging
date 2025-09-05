use core::{ops::RangeInclusive, ptr::NonNull};

use x86_64::{
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{PageTable, PageTableIndex},
};

use crate::*;

use super::page_table_with_level::{PageTableLevel, PageTableWithLevelMut};

#[derive(Debug)]
pub struct KernelL4Data {
    is_referenced: bool,
}

#[derive(Debug)]
pub(super) enum L4Type {
    User,
    Kernel(KernelL4Data),
}

impl L4Type {
    pub fn l4_managed_entry_range(&self) -> RangeInclusive<PageTableIndex> {
        match self {
            Self::User => PageTableIndex::new(0)..=PageTableIndex::new(255),
            Self::Kernel(_) => PageTableIndex::new(256)..=PageTableIndex::new(511),
        }
    }

    pub fn can_create_new_l4_entries(&self) -> bool {
        match self {
            Self::User => true,
            Self::Kernel(KernelL4Data { is_referenced }) => !is_referenced,
        }
    }
}

#[derive(Debug)]
pub struct ManagedL4PageTable {
    pub(super) frame: Owned4KibFrame,
    pub(super) _type: L4Type,
    pub(super) config: PagingConfig,
}

/// # Safety
/// Frame will be zeroed
unsafe fn init_page_table(frame: &mut Owned4KibFrame, config: &PagingConfig) {
    let ptr = NonNull::new(
        frame
            .start_address()
            .to_virt(config)
            .as_mut_ptr::<PageTable>(),
    )
    .unwrap();
    // We use `write_bytes` so that we don't put the 4 KiB page table on the stack, which causes stack overflows.
    unsafe {
        ptr.write_bytes(0, 1);
    }
}

impl PagingConfig {
    /// Create a new top level page table meant to only be accessed by the kernel.
    /// You will only be allowed to use the higher half of the virtual address space.
    ///
    /// This method also zeroes the frame.
    pub fn new_kernel(self, mut frame: Owned4KibFrame) -> ManagedL4PageTable {
        unsafe { init_page_table(&mut frame, &self) };
        ManagedL4PageTable {
            frame,
            _type: L4Type::Kernel(KernelL4Data {
                is_referenced: false,
            }),
            config: self,
        }
    }
}

impl ManagedL4PageTable {
    /// Create a new top level page table meant to be accessed by a process.
    /// You will only be able to use the lower half of the virtual address space.
    ///
    /// This method also zeroes the frame.
    pub fn new_user(&mut self, mut frame: Owned4KibFrame) -> Self {
        match &mut self._type {
            L4Type::User => {
                panic!("self must be a kernel's l4 frame to copy from it")
            }
            L4Type::Kernel(KernelL4Data { is_referenced }) => {
                *is_referenced = true;
            }
        };
        unsafe { init_page_table(&mut frame, &self.config) };
        let mut lower_half = Self {
            frame,
            _type: L4Type::User,
            config: self.config,
        };
        let range_to_copy = self._type.l4_managed_entry_range();
        let kernel_page_table = unsafe { self.page_table().as_mut() };
        let user_page_table = unsafe { lower_half.page_table().as_mut() };
        for index in range_to_copy {
            user_page_table[index].clone_from(&kernel_page_table[index]);
        }
        lower_half
    }

    /// If you choose to manually modify page table entries, be careful, because it could create valid page tables that will cause problems because this crate doesn't expect handle.
    pub fn page_table(&mut self) -> NonNull<PageTable> {
        NonNull::new(
            self.frame
                .start_address()
                .to_virt(&self.config)
                .as_mut_ptr::<PageTable>(),
        )
        .unwrap()
    }

    pub(super) fn table_mut(&mut self) -> PageTableWithLevelMut {
        PageTableWithLevelMut {
            page_table: self.page_table(),
            level: PageTableLevel::L4,
            l4: self,
        }
    }

    /// # Safety
    /// Changes Cr3 value
    pub unsafe fn switch_to(&self, flags: Cr3Flags) {
        unsafe { Cr3::write(self.frame.0, flags) };
    }

    pub fn frame(&self) -> &Owned4KibFrame {
        &self.frame
    }
}
