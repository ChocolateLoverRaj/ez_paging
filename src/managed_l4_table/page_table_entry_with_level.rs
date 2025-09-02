use core::ptr::NonNull;

use raw_cpuid::CpuId;
use x86_64::structures::paging::{
    PageTable, PageTableFlags, PhysFrame, page_table::PageTableEntry,
};

use super::{ConfigurableFlags, ManagedL4PageTable, PageTableLevel, PageTableWithLevelMut};
use crate::{managed_l4_table::L4Type, *};

#[derive(Debug)]
pub struct PageTableEntryWithLevelMut<'a> {
    pub(super) entry: &'a mut PageTableEntry,
    pub(super) level: PageTableLevel,
    pub(super) l4: &'a ManagedL4PageTable,
}

#[derive(Debug)]
pub enum SetFrameError {
    /// Either the page table is a L4 table (you can't map a 512 GiB frame) or the frame size is incompatible with the table level.
    NotAllowed,
    /// This CPU cannot have 1 GiB page sizes
    PageSizeNotSupported,
}

#[derive(Debug)]
pub enum SetTableError {
    /// This page table is a L1 table and L1 entries don't point to another page table.
    IsL1,
}

#[derive(Debug)]
pub enum GetTableError {
    /// This is a L1 table and cannot point to another table
    IsL1,
    /// This entry is not mapped
    NotMapped,
    /// This entry is mapped, but not mapped to a table
    MappedToFrame,
}

#[derive(Debug)]
pub enum UnmapFrameError {
    IsL4,
    NotPresent,
    IsPageTable,
}

#[derive(Debug)]
pub enum SetFlagsError {
    IsL4,
    NotPresent,
    IsPageTable,
}

impl PageTableEntryWithLevelMut<'_> {
    pub fn is_empty(&self) -> bool {
        self.entry.is_unused()
    }

    fn page_size(&self) -> Option<PageSize> {
        match self.level {
            PageTableLevel::L1 => Some(PageSize::_4KiB),
            PageTableLevel::L2 => Some(PageSize::_2MiB),
            PageTableLevel::L3 => Some(PageSize::_1GiB),
            PageTableLevel::L4 => None,
        }
    }

    fn generate_flags(&self, configurable_flags: ConfigurableFlags) -> PageTableFlags {
        let mut flags = PageTableFlags::PRESENT | self.l4.config.pat
            .get_page_table_flags(configurable_flags.pat_memory_type, self.page_size().unwrap())
            .expect("There are only 6 memory types and 8 slots, so all memory types should be present in the slots");
        if !matches!(self.level, PageTableLevel::L1) {
            flags |= PageTableFlags::HUGE_PAGE
        }
        if configurable_flags.writable {
            flags |= PageTableFlags::WRITABLE;
        }
        if !configurable_flags.executable {
            flags |= PageTableFlags::NO_EXECUTE;
        }
        match &self.l4._type {
            L4Type::User => {
                flags |= PageTableFlags::USER_ACCESSIBLE;
            }
            L4Type::Kernel(_) => {
                flags |= PageTableFlags::GLOBAL;
            }
        };
        flags
    }

    pub fn set_frame(
        &mut self,
        frame: Frame,
        flags: ConfigurableFlags,
    ) -> Result<(), SetFrameError> {
        let level_frame_match = match self.level {
            PageTableLevel::L1 => matches!(frame.size(), PageSize::_4KiB),
            PageTableLevel::L2 => matches!(frame.size(), PageSize::_2MiB),
            PageTableLevel::L3 => matches!(frame.size(), PageSize::_1GiB),
            PageTableLevel::L4 => false,
        };
        if !level_frame_match {
            return Err(SetFrameError::NotAllowed);
        }
        if frame.size() == PageSize::_1GiB
            && !CpuId::new()
                .get_extended_processor_and_feature_identifiers()
                .is_some_and(|info| info.has_1gib_pages())
        {
            return Err(SetFrameError::PageSizeNotSupported);
        }
        self.entry
            .set_addr(frame.start_addr(), self.generate_flags(flags));
        Ok(())
    }

    /// Returns the frame that was unmapped
    pub fn unmap_frame(&mut self) -> Result<Frame, UnmapFrameError> {
        let frame_size = self
            .level
            .target_frame_size()
            .ok_or(UnmapFrameError::IsL4)?;
        if !self.entry.flags().contains(PageTableFlags::PRESENT) {
            return Err(UnmapFrameError::NotPresent);
        }
        if !self.entry.flags().contains(PageTableFlags::HUGE_PAGE)
            && !matches!(frame_size, PageSize::_4KiB)
        {
            return Err(UnmapFrameError::IsPageTable);
        }
        let start_addr = self.entry.addr();
        self.entry.set_unused();
        Ok(Frame::new(start_addr, frame_size).unwrap())
    }

    /// Only sets flags for pointing to a frame, not pointing to a table
    pub fn set_flags(&mut self, flags: ConfigurableFlags) -> Result<(), SetFlagsError> {
        let page_size = self.level.target_frame_size().ok_or(SetFlagsError::IsL4)?;
        if !self.entry.flags().contains(PageTableFlags::PRESENT) {
            return Err(SetFlagsError::NotPresent);
        }
        if !self.entry.flags().contains(PageTableFlags::HUGE_PAGE)
            && !matches!(page_size, PageSize::_4KiB)
        {
            return Err(SetFlagsError::IsPageTable);
        }
        self.entry.set_flags(self.generate_flags(flags));
        Ok(())
    }
}

impl<'a> PageTableEntryWithLevelMut<'a> {
    /// This method also zeroes the frame
    pub fn set_page_table(
        self,
        frame: PhysFrame,
    ) -> Result<PageTableWithLevelMut<'a>, SetTableError> {
        let page_table_level = self.level.sub_level().ok_or(SetTableError::IsL1)?;
        if self.level == PageTableLevel::L4 && !self.l4._type.can_create_new_l4_entries() {
            panic!(
                "Cannot create new L3 pages because the kernel page table would be out of sync with user page tables"
            )
        }
        let ptr = NonNull::new(
            frame
                .start_address()
                .to_virt(&self.l4.config)
                .as_mut_ptr::<PageTable>(),
        )
        .unwrap();
        unsafe { ptr.write_bytes(0, 1) };

        self.entry.set_frame(
            frame,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
        );
        Ok(PageTableWithLevelMut {
            page_table: ptr,
            level: page_table_level,
            l4: self.l4,
        })
    }

    pub fn get_page_table_mut(self) -> Result<PageTableWithLevelMut<'a>, GetTableError> {
        let page_table_level = self.level.sub_level().ok_or(GetTableError::IsL1)?;
        if self.entry.is_unused() {
            return Err(GetTableError::NotMapped);
        }
        if self.entry.flags().contains(PageTableFlags::HUGE_PAGE) {
            return Err(GetTableError::MappedToFrame);
        }
        let frame = self.entry.frame(false).unwrap();
        let ptr = NonNull::new(
            frame
                .start_address()
                .to_virt(&self.l4.config)
                .as_mut_ptr::<PageTable>(),
        )
        .unwrap();
        Ok(PageTableWithLevelMut {
            page_table: ptr,
            level: page_table_level,
            l4: self.l4,
        })
    }
}
