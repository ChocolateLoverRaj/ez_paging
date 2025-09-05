//! This crate is meant to be used in kernels.
//! It currently only supports `x86_64` with 4-level paging.
//! This crate assumes that you will have globally mapped kernel mappings in the higher half,
//! and process mappings in the lower half.
//! Get started by constructing a [`PagingConfig`],
#![no_std]
use addr_translation::*;
pub use frame::*;
pub use managed_l4_table::*;
pub use managed_pat::*;
pub use max_page_size::*;
pub use owned_4kib_frame::*;
pub use page::*;
pub use page_size::*;
pub use paging_config::*;
pub use virtual_offset::*;

mod addr_translation;
mod frame;
mod managed_l4_table;
mod managed_pat;
mod max_page_size;
mod owned_4kib_frame;
mod page;
mod page_size;
mod paging_config;
mod virtual_offset;
