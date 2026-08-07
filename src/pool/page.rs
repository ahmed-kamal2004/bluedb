use std::fmt::Display;

use anyhow::Result;
use zerocopy::{Immutable, IntoBytes, TryFromBytes};

use super::super::constants::PAGE_SIZE;
use super::super::heap::heap::HeapPageView;
use super::super::page::btree::BtreePageView;
use super::super::page::mngmnt::MngmntPageView;

#[repr(u8)]
#[derive(IntoBytes, TryFromBytes, Immutable, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageTp {
    /// Heap Page
    HpPg,
    /// B+ Tree Page
    BppPg,
    /// Management Page
    MngmntPg,
}

#[derive(Debug, Clone, Copy)]
pub struct Page {
    pub data: [u8; PAGE_SIZE],
}

impl Page {
    pub fn new() -> Self {
        Page {
            data: [0; PAGE_SIZE],
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut data = [0; PAGE_SIZE];
        data.copy_from_slice(&bytes[0..PAGE_SIZE]);
        Page { data }
    }

    pub fn to_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn get_pg_tp(&self) -> PageTp {
        match self.data[0] {
            0 => PageTp::HpPg,
            1 => PageTp::BppPg,
            2 => PageTp::MngmntPg,
            _ => panic!("Invalid page type"),
        }
    }

    pub fn set_pg_tp(&mut self, pg_tp: PageTp) {
        self.data[0] = match pg_tp {
            PageTp::HpPg => 0,
            PageTp::BppPg => 1,
            PageTp::MngmntPg => 2,
        };
    }

    pub fn create_heap_page_view(&mut self) -> Result<HeapPageView<'_>> {
        match self.get_pg_tp() {
            PageTp::HpPg => Ok(HeapPageView::new(self)),
            _ => Err(anyhow::anyhow!("Page is not a Heap Page")),
        }
    }

    pub fn create_btree_page_view(&mut self) -> Result<BtreePageView<'_>> {
        match self.get_pg_tp() {
            PageTp::BppPg => Ok(BtreePageView::new(self)),
            _ => Err(anyhow::anyhow!("Page is not a B+ Tree Page")),
        }
    }

    pub fn create_mngmnt_page_view(&mut self) -> Result<MngmntPageView<'_>> {
        match self.get_pg_tp() {
            PageTp::MngmntPg => Ok(MngmntPageView::new(self)),
            _ => Err(anyhow::anyhow!("Page is not a Management Page")),
        }
    }
}
