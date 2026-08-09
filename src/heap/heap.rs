use std::fmt::Display;

use crate::page::traits::Initialize;

use super::super::pool::page::Page;
use super::super::pool::page::PageTp;
use anyhow::Result;
use zerocopy::Immutable;
use zerocopy::IntoBytes;
use zerocopy::KnownLayout;
use zerocopy::TryFromBytes;

use super::super::constants::{
    FREE_SPACE_HEADER_SIZE, PAGE_HEADER_SIZE, PAGE_SIZE, ROW_POINTER_SIZE,
};

pub struct HeapPageView<'a> {
    pub page: &'a mut Page,
    pub nxt_rw_ptr_ofst: i32,
}

impl<'a> HeapPageView<'a> {
    pub fn new(page: &'a mut Page) -> Self {
        HeapPageView {
            page,
            nxt_rw_ptr_ofst: -1,
        }
    }
}

/*
Heap Page Structure:

- Page Header (11 bytes total):
    - Page Type (1 byte): Indicates that this is a heap page.
    - Free Space total (2 bytes): The total amount of free space available in the page.
    - Next Free Space Offset (2 bytes): The offset to the next free space in the page.
    - Next Row Pointer Offset (2 bytes): The offset to the next row pointer in the page.
    - Last Transaction ID (4 bytes): The ID of the last transaction that modified this page.

- Free Space Header (4 bytes total):
    - Next Free Space Offset (2 bytes): The offset to the next free space in the page.
    - Free Space Size (2 bytes): The size of the free space.


- Row Pointer (2 bytes): The offset to the row data in the page.

- Row Header (10 bytes total):
    - TMIN (4 bytes): The minimum transaction ID that can see this row.
    - TMAX (4 bytes): The maximum transaction ID that can see this row.
    - Data Length (2 bytes): The length of the row data.

- Row Data (variable length):
    - Field Length (2 bytes): The length of each field in the row data.
    - Field Data (variable length): The actual data for each field in the row.
    - Loop



* If the page is newly created, we must call the initialize() method to set up the page header and free space header. This ensures that the page is in a valid state before any rows are inserted.


Diagrams: (Generated with the help of llms)

///
///  0                PAGE_HEADER_SIZE                                  PAGE_SIZE
///  ┌──────────────┬───────────────┬─────────────────┬─────────────────┐
///  │  Page Header │ Row Pointers  │   Free Space     │    Row Data     │
///  │  (fixed size)│  (grow  →)    │  (shrinks as     │   (← grows)     │
///  │              │               │   both sides use │                 │
///  │              │               │   it up)         │                 │
///  └──────────────┴───────────────┴─────────────────┴─────────────────┘
///                  ^                                  ^
///                  first row pointer                  last row's data
///                  at PAGE_HEADER_SIZE                 ends at PAGE_SIZE
///
///  - Row Pointers grow FORWARD (toward higher offsets) as rows are added.
///  - Row Data grows BACKWARD (toward lower offsets) as rows are added.
///  - The gap between them is free space, tracked as a linked list of
///    FreeSpaceHeader chunks (see below).


/// PageHeader
///
///  byte:  0        1        2        3        4        5        6        7        8        9        10
///        ┌────────┬─────────────────┬─────────────────┬─────────────────┬───────────────────────────┐
///        │  tp    │ free_space_total│ next_free_space_ │ next_row_pointer│  last_transaction_id      │
///        │ (u8)   │      (u16)      │  offset  (u16)   │  offset  (u16)  │         (u32)             │
///        └────────┴─────────────────┴─────────────────┴─────────────────┴───────────────────────────┘
///         FrameType   bytes of free    where to find      where to find     txn id that last wrote
///         discriminant  space left      the head of the    the most-        to this page
///                                       free-space chain    recently-added
///                                                            row pointer

/// FreeSpaceHeader
/// Sits at the START of each free chunk; chunks form a singly-linked list.
///
///  byte:  0        1        2        3
///        ┌─────────────────┬─────────────────┐
///        │ next_free_space_│ free_space_size  │
///        │  offset  (u16)  │      (u16)       │
///        └─────────────────┴─────────────────┘
///
/// Chain shape (0 terminates the list, same convention as next_row_pointer_offset):
///
///   page_header.next_free_space_offset
///                 │
///                 ▼
///        ┌─────────────────┐        ┌─────────────────┐
///        │ FreeSpaceHeader │──next─▶│ FreeSpaceHeader │──next──▶ 0 (end)
///        │   chunk A       │        │   chunk B       │
///        └─────────────────┘        └─────────────────┘


/// RowPointer
/// One per row, stored in the growing array right after the page header.
///
///  byte:  0        1
///        ┌─────────────────┐
///        │   row_offset    │───▶ points to where this row's RowHeader
///        │      (u16)      │     actually lives (down in the Row Data area)
///        └─────────────────┘
///
///  Row Pointers array (indexed from PAGE_HEADER_SIZE, growing forward):
///
///   PAGE_HEADER_SIZE
///        │
///        ▼
///   ┌─────────┬─────────┬─────────┬─────  ...
///   │ Pointer │ Pointer │ Pointer │
///   │  → row0 │ → row1  │ → row2  │
///   └─────────┴─────────┴─────────┴─────  ...

/// RowHeader
/// Followed immediately by this row's Row Data (variable length).
///
///  byte:  0                 4                 8        9        10
///        ┌─────────────────┬─────────────────┬─────────────────┐
///        │      tmin       │      tmax       │     length      │
///        │      (u32)      │      (u32)      │      (u16)      │
///        └─────────────────┴─────────────────┴─────────────────┘
///         MVCC: txn that     MVCC: txn that     total bytes of
///         created this row   deleted this row   Row Data that follows
///                             (0 = not deleted)   (all fields combined)
///
/// Row Data — immediately after RowHeader, one repeated unit per field:
///
///  ┌─────────────────┬───────────────────────┬─────────────────┬──────────────
///  │  Field Length    │      Field Data       │  Field Length   │  Field Data...
///  │      (u16)       │  (Field Length bytes) │      (u16)      │
///  └─────────────────┴───────────────────────┴─────────────────┴──────────────
///
///  [RowHeader][ FieldLen | FieldData ][ FieldLen | FieldData ] ... (repeats
///  until the running total equals RowHeader.length)

/// How a RowPointer resolves to actual row content:
///
///  Row Pointers area                         Row Data area (grows ← from PAGE_SIZE)
///  ┌─────────┐                                              ┌───────────┬─────────────┐
///  │ Pointer │── row_offset ───────────────────────────────▶│ RowHeader │  Row Data   │
///  │ (2 B)   │                                               │ (10 B)    │ (variable)  │
///  └─────────┘                                              └───────────┴─────────────┘
///                                                             tmin/tmax/  [len|data]...
///                                                             length

*/

/// The HeapPageHeader struct represents the header of a heap page in the database.
#[repr(C, packed)]
#[derive(IntoBytes, TryFromBytes, Immutable, KnownLayout)]
pub struct HeapPageHeader {
    pub tp: PageTp,
    pub fr_spc_tl: u16,
    pub nxt_fr_spc_ofst: u16,
    pub nxt_rw_ptr_ofst: u16,
    pub lst_txn: u32,
}

/// The FreeSpaceHeader struct represents the header of a free space block in the heap page.
#[repr(C, packed)]
#[derive(IntoBytes, TryFromBytes, Immutable, KnownLayout)]
pub struct FreeSpaceHeader {
    pub nxt_fr_spc_ofst: u16,
    pub fr_spc_sz: u16,
}

/// The RowPointer struct represents a pointer to a row in the heap page.
#[repr(C, packed)]
#[derive(IntoBytes, TryFromBytes, Immutable, KnownLayout)]
pub struct RowPointer {
    pub rw_ofst: u16,
}

/// The RowHeader struct represents the header of a row in the heap page.
#[repr(C, packed)]
#[derive(IntoBytes, TryFromBytes, Immutable, KnownLayout)]
pub struct RowHeader {
    pub tmin: u32,
    pub tmax: u32,
    pub len: u16,
}

/// The FieldData struct represents a field in the row data of the heap page.
#[derive(Clone, PartialEq, Eq)]
pub struct FieldData {
    pub len: u16,
    pub data: Vec<u8>,
}

pub struct Row {
    pub header: RowHeader,
    pub fields: Vec<FieldData>,
}

impl Display for Row {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tmin = self.header.tmin;
        let tmax = self.header.tmax;
        let len = self.header.len;
        write!(
            f,
            "Row {{ tmin: {}, tmax: {}, length: {}, fields: [",
            tmin, tmax, len
        )?;
        for field in &self.fields {
            write!(f, "{{ length: {}, data: {:?} }}, ", field.len, field.data)?;
        }
        write!(f, "] }}")
    }
}

impl Initialize for HeapPageView<'_> {
    /// Initializes the heap page with the given transaction ID.
    /// This method sets up the page header and the first free space header.
    /// It should be called when a new heap page is created to ensure that the page is in a valid state before any rows are inserted.
    fn initialize(&mut self, txn_id: u32) -> Result<()> {
        // Add the page header.
        let pg_hdr = HeapPageHeader {
            tp: PageTp::HpPg,
            fr_spc_tl: (PAGE_SIZE - PAGE_HEADER_SIZE) as u16,
            nxt_fr_spc_ofst: PAGE_HEADER_SIZE as u16,
            nxt_rw_ptr_ofst: 0,
            lst_txn: txn_id,
        };

        let pg_hdr_bytes = pg_hdr.as_bytes();

        if pg_hdr_bytes.len() != PAGE_HEADER_SIZE {
            return Err(anyhow::anyhow!("Page header size mismatch"));
        }

        self.page.data[0..PAGE_HEADER_SIZE].copy_from_slice(pg_hdr_bytes);

        // Add the first free space header.
        let fr_spc_hdr = FreeSpaceHeader {
            nxt_fr_spc_ofst: 0,
            fr_spc_sz: (PAGE_SIZE - PAGE_HEADER_SIZE - FREE_SPACE_HEADER_SIZE) as u16,
        };

        let fr_spc_hdr_bytes = fr_spc_hdr.as_bytes();

        if fr_spc_hdr_bytes.len() != FREE_SPACE_HEADER_SIZE {
            return Err(anyhow::anyhow!("Free space header size mismatch"));
        }

        self.page.data[PAGE_HEADER_SIZE..PAGE_HEADER_SIZE + FREE_SPACE_HEADER_SIZE]
            .copy_from_slice(fr_spc_hdr_bytes);

        // page is now initialized with the page header and the first free space header.
        Ok(())
    }
}

/// Read-only iterator for HeapPageView
pub struct HeapPageViewRefIter<'a, 'b> {
    view: &'b HeapPageView<'a>,
    nxt_rw_ptr_ofst: i32,
}

impl<'a, 'b> Iterator for HeapPageViewRefIter<'a, 'b> {
    type Item = Row;

    fn next(&mut self) -> Option<Self::Item> {
        // First time initialization
        if self.nxt_rw_ptr_ofst == -1 {
            let pg_hdr = self.view.get_header().expect("Failed to read page header");
            self.nxt_rw_ptr_ofst = pg_hdr.nxt_rw_ptr_ofst as i32;
        }

        if self.nxt_rw_ptr_ofst < PAGE_HEADER_SIZE as i32 {
            return None;
        }

        // Reads the row cleanly through the shared view reference
        let rw = self
            .view
            .read_row(self.nxt_rw_ptr_ofst as usize)
            .expect("Failed to read row");
        self.nxt_rw_ptr_ofst -= ROW_POINTER_SIZE as i32;
        Some(rw)
    }
}

impl<'a, 'b> IntoIterator for &'b HeapPageView<'a> {
    type Item = Row;
    type IntoIter = HeapPageViewRefIter<'a, 'b>;

    fn into_iter(self) -> Self::IntoIter {
        HeapPageViewRefIter {
            view: self,
            nxt_rw_ptr_ofst: -1,
        }
    }
}

impl Display for HeapPageHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tp = self.tp;
        let fr_spc_tl = self.fr_spc_tl;
        let nxt_fr_spc_ofst = self.nxt_fr_spc_ofst;
        let nxt_rw_ptr_ofst = self.nxt_rw_ptr_ofst;
        let lst_txn = self.lst_txn;
        write!(
            f,
            "HeapPageHeader {{ page_type: {:?}, free_space_total: {}, next_free_space_offset: {}, next_row_pointer_offset: {}, last_transaction_id: {} }}",
            tp, fr_spc_tl, nxt_fr_spc_ofst, nxt_rw_ptr_ofst, lst_txn
        )
    }
}

impl Display for HeapPageView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pg_hdr = self.get_header().expect("Failed to read page header");

        write!(f, "HeapPageView {{ header: {}, rows: [", pg_hdr)?;

        let iter_vw = self.into_iter();

        for row in iter_vw {
            write!(f, "{}, ", row)?;
        }

        write!(f, "] }}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{
        PAGE_HEADER_SIZE, PAGE_SIZE, ROW_DATA_LENGTH_SIZE, ROW_HEADER_SIZE, ROW_POINTER_SIZE,
    };
    use crate::heap::insert::Insert;
    use crate::page::traits::Initialize;
    use crate::pool::page::{Page, PageTp};

    fn field(data: &[u8]) -> FieldData {
        FieldData {
            len: data.len() as u16,
            data: data.to_vec(),
        }
    }

    fn new_heap_page() -> Page {
        let mut page = Page::new();
        page.set_pg_tp(PageTp::HpPg);
        page
    }

    #[test]
    fn helper_accessors_read_inserted_row_and_header() -> Result<()> {
        let mut page = new_heap_page();
        let mut view = page.create_heap_page_view()?;
        view.initialize(11)?;

        let row = vec![field(b"hello"), field(b"world")];
        view.insert_row(&row, 22)?;

        for row in &view {
            assert_eq!(row.fields[0].data, b"hello");
            assert_eq!(row.fields[1].data, b"world");
            assert!(row.header.tmin == 22);
            assert!(row.header.tmax == 0);
        }
        Ok(())
    }

    #[test]
    fn insert_multiple_rows_and_traverse() -> Result<()> {
        let mut page = new_heap_page();
        let mut view = page.create_heap_page_view()?;
        view.initialize(11)?;

        let row1 = vec![field(b"hello"), field(b"world")];
        view.insert_row(&row1, 22)?;

        let row2 = vec![field(b"foo"), field(b"bar")];
        view.insert_row(&row2, 23)?;

        let mut iter = view.into_iter();

        if let Some(row) = iter.next() {
            assert_eq!(row.fields[0].data, b"foo");
            assert_eq!(row.fields[1].data, b"bar");
            assert!(row.header.tmin == 23);
            assert!(row.header.tmax == 0);
        } else {
            panic!("Expected a row but got None");
        }

        if let Some(row) = iter.next() {
            assert_eq!(row.fields[0].data, b"hello");
            assert_eq!(row.fields[1].data, b"world");
            assert!(row.header.tmin == 22);
            assert!(row.header.tmax == 0);
        } else {
            panic!("Expected a row but got None");
        }

        assert!(iter.next().is_none());

        Ok(())
    }

    #[test]
    fn insert_row_returns_error_when_page_has_not_enough_space() -> Result<()> {
        let mut page = new_heap_page();
        let mut view = page.create_heap_page_view()?;
        view.initialize(100)?;

        let too_large_payload = vec![0_u8; crate::constants::PAGE_SIZE];
        let row = vec![field(too_large_payload.as_slice())];

        let result = view.insert_row(&row, 1);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn insert_row_returns_error_when_page_is_not_initialized() -> Result<()> {
        let mut page = new_heap_page();
        let mut view = page.create_heap_page_view()?;

        let row = vec![field(b"hello"), field(b"world")];
        let result = view.insert_row(&row, 1);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn insert_and_check_page_header_updates() -> Result<()> {
        let mut page = new_heap_page();
        let mut view = page.create_heap_page_view()?;
        view.initialize(11)?;

        let row = vec![field(b"hello"), field(b"world")];
        view.insert_row(&row, 22)?;
        view.insert_row(&row, 23)?;
        view.insert_row(&row, 24)?;

        println!("{}", view);

        let header = view.get_header()?;
        let fr_spc_tl = header.fr_spc_tl;
        assert_eq!(
            fr_spc_tl,
            (PAGE_SIZE
                - PAGE_HEADER_SIZE
                - 3 * (ROW_POINTER_SIZE
                    + ROW_HEADER_SIZE
                    + row
                        .iter()
                        .map(|f| f.data.len() + ROW_DATA_LENGTH_SIZE)
                        .sum::<usize>())) as u16
        );
        Ok(())
    }

    #[test]
    fn free_space_in_page_header_equal_in_free_space_headers() -> Result<()> {
        let mut page = new_heap_page();
        let mut view = page.create_heap_page_view()?;
        view.initialize(11)?;

        let row = vec![field(b"hello"), field(b"world")];
        view.insert_row(&row, 22)?;
        view.insert_row(&row, 23)?;

        let header = view.get_header()?;
        let fr_spc_tl = header.fr_spc_tl;

        // Check that the free space total in the page header matches the sum of free space sizes in the free space headers
        let mut total_free_space = 0;
        let mut next_free_space_offset = header.nxt_fr_spc_ofst as usize;

        while next_free_space_offset != 0 {
            let free_space_header = view.get_fr_spc_hdr(next_free_space_offset)?;
            total_free_space += free_space_header.fr_spc_sz as usize;
            next_free_space_offset = free_space_header.nxt_fr_spc_ofst as usize;
        }

        assert_eq!(fr_spc_tl as usize, total_free_space + 2 * ROW_POINTER_SIZE);
        Ok(())
    }

    #[test]
    fn iterating_empty_page_returns_no_rows() -> Result<()> {
        let mut page = new_heap_page();
        let mut view = page.create_heap_page_view()?;
        view.initialize(11)?;
        // no inserts at all

        let mut iter = view.into_iter();
        assert!(iter.next().is_none());
        Ok(())
    }

    #[test]
    fn iterating_page_with_one_row_returns_that_row() -> Result<()> {
        let mut page = new_heap_page();
        let mut view = page.create_heap_page_view()?;
        view.initialize(11)?;

        let row = vec![field(b"hello"), field(b"world")];
        view.insert_row(&row, 22)?;

        let mut iter = view.into_iter();
        if let Some(row) = iter.next() {
            assert_eq!(row.fields[0].data, b"hello");
            assert_eq!(row.fields[1].data, b"world");
            assert!(row.header.tmin == 22);
            assert!(row.header.tmax == 0);

            assert!(iter.next().is_none());

            Ok(())
        } else {
            panic!("Expected a row but got None");
        }
    }
}
