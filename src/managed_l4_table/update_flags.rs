use x86_64::instructions::tlb::flush;

use crate::*;

use super::{GetTableError, ManagedL4PageTable, SetFlagsError};

#[derive(Debug)]
pub enum UpdateFlagsError {
    GetTable(GetTableError),
    SetFlags(SetFlagsError),
}

impl ManagedL4PageTable {
    /// # Safety
    /// Changing flags could cause page faults, or worse, let user mode access memory it shouldn't be allowed to.
    pub unsafe fn update_flags(
        &mut self,
        page: Page,
        flags: ConfigurableFlags,
    ) -> Result<(), UpdateFlagsError> {
        let l4 = self.table_mut();
        let l3 = l4
            .entry_mut(page.start_addr().p4_index())
            .get_page_table_mut()
            .map_err(UpdateFlagsError::GetTable)?;
        let l3_entry = l3.entry_mut(page.start_addr().p3_index());
        let mut entry: super::PageTableEntryWithLevelMut<'_> = if let PageSize::_1GiB = page.size()
        {
            l3_entry
        } else {
            let l2 = l3_entry
                .get_page_table_mut()
                .map_err(UpdateFlagsError::GetTable)?;
            let l2_entry = l2.entry_mut(page.start_addr().p2_index());
            if let PageSize::_2MiB = page.size() {
                l2_entry
            } else {
                let l1 = l2_entry
                    .get_page_table_mut()
                    .map_err(UpdateFlagsError::GetTable)?;
                l1.entry_mut(page.start_addr().p1_index())
            }
        };
        entry.set_flags(flags).map_err(UpdateFlagsError::SetFlags)?;
        flush(page.start_addr());
        Ok(())
    }
}
