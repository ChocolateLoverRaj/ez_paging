#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "bincode", derive(bincode::Encode, bincode::Decode))]
pub enum PageSize {
    _4KiB,
    _2MiB,
    _1GiB,
}

impl PageSize {
    pub const fn byte_len(self) -> usize {
        self.byte_len_u64() as usize
    }

    pub const fn byte_len_u64(self) -> u64 {
        match self {
            PageSize::_4KiB => 0x1000,
            PageSize::_2MiB => 512 * 0x1000,
            PageSize::_1GiB => 512 * 512 * 0x1000,
        }
    }
}
