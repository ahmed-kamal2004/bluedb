use anyhow::Result;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, TryFromBytes};

/// The physical normal page in disk
/// Page layout:
/// [Page Header][Row Header][Row Data]
/// Page Header (23 bytes):
///         [Page Type (1 byte)]
/// 
///         [Free Space Total (2 bytes)]
///         [Next Free Space Offset (2 bytes)]
///         [Next Row Pointer Offset (2 bytes)]
/// 
///         [Last transaction ID (4 bytes)]
/// 
///         [Page ID (4 bytes)]
///         [Next Page ID (4 bytes)]
///         [Prev Page ID (4 bytes)]
/// 
/// Free Space Header (4 bytes):
///         [Next Free Space Offset (2 bytes)]
///         [Free Space Size (2 bytes)]
/// 
/// Row Pointer (4 bytes):
///      [Row Offset (2 bytes)]
///      [Next Row Pointer Offset (2 bytes)]
/// 
/// Row header layout (10 bytes):
///      [Tmin (4 bytes)][Tmax (4 bytes)][Length (2 bytes)] + [Row Data (variable length)]
/// 
/// Row Data layout:
///      [Field Length (2 bytes)][Field Data (variable length)][Field Length (2 bytes)][Field Data (variable length)]...
/// 
/// TODO: enable HOT using cid


// Page-wide constants
const PAGE_SIZE: usize = 4096;

// Page header layout constants
const PAGE_HEADER_SIZE: usize = 23;
const PAGE_TYPE_SIZE: usize = 1;
const FREE_SPACE_TOTAL_SIZE: usize = 2;
const NEXT_FREE_SPACE_OFFSET_SIZE: usize = 2;
const NEXT_ROW_POINTER_OFFSET_SIZE: usize = 2;
const LAST_TRANSACTION_ID_SIZE: usize = 4;
const PAGE_ID_SIZE: usize = 4;
const NEXT_PAGE_ID_SIZE: usize = 4;
const PREV_PAGE_ID_SIZE: usize = 4;

// Free space header layout constants
const FREE_SPACE_HEADER_SIZE: usize = 4;
// const NEXT_FREE_SPACE_OFFSET_SIZE: usize = 2; // declared above

// Row pointer layout constants
const ROW_POINTER_SIZE: usize = 4;
const ROW_OFFSET_SIZE: usize = 2;
// const NEXT_ROW_POINTER_OFFSET_SIZE: usize = 2; // declared above

// Row header layout constants
const ROW_HEADER_SIZE: usize = 10;
const ROW_TMIN_SIZE: usize = 4;
const ROW_TMAX_SIZE: usize = 4;
const ROW_DATA_LENGTH_SIZE: usize = 2;

// Row data layout constants
const FIELD_LENGTH_SIZE: usize = 2;

#[repr(u8)]
#[derive(Immutable, TryFromBytes, KnownLayout, IntoBytes)]

pub enum FrameType {
    Normal = 0,
    Index = 1,
}

#[repr(C, packed)]
#[derive(Immutable, TryFromBytes, KnownLayout, IntoBytes)]
pub struct PageHeader {
    pub tp: FrameType,
    pub free_space_total: u16,
    pub next_free_space_offset: u16,
    pub next_row_pointer_offset: u16,
    pub last_transaction_id: u32,
    pub page_id: u32,
    pub next_page_id: u32,
    pub prev_page_id: u32,
}

#[repr(C, packed)]
#[derive(Immutable, FromBytes, KnownLayout, IntoBytes)]
pub struct FreeSpaceHeader {
    pub next_free_space_offset: u16,
    pub free_space_size: u16,
}

#[repr(C, packed)]
#[derive(Immutable, FromBytes, KnownLayout, IntoBytes)]
pub struct RowPointer {
    pub row_offset: u16,
    pub next_row_pointer_offset: u16,
}

#[repr(C, packed)]
#[derive(Immutable, FromBytes, KnownLayout, IntoBytes)]
pub struct RowHeader {
    pub tmin: u32,
    pub tmax: u32,
    pub length: u16,
}

pub struct FieldData {
    pub length: u16,
    pub data: Vec<u8>,
}

pub struct Frame {
    pub data: [u8; PAGE_SIZE],
}

impl Frame {

    // Page initialization and header management
    pub fn initialize(&mut self, frame_type: FrameType, page_id: u32, txn_id: u32) {

        // Add the page header
        let page_header = PageHeader {
            tp: frame_type,
            free_space_total: (PAGE_SIZE - PAGE_HEADER_SIZE) as u16,
            next_free_space_offset: PAGE_HEADER_SIZE as u16,
            next_row_pointer_offset: PAGE_HEADER_SIZE as u16,
            last_transaction_id: txn_id,
            page_id,
            next_page_id: 0,
            prev_page_id: 0,
        };

        // Add the first free space header
        let free_space_header = FreeSpaceHeader {
            next_free_space_offset: 0 as u16,
            free_space_size: (PAGE_SIZE - PAGE_HEADER_SIZE) as u16,
        };

        let page_header_bytes = page_header.as_bytes();
        self.data[0..PAGE_HEADER_SIZE].copy_from_slice(page_header_bytes);
        self.data[PAGE_HEADER_SIZE..PAGE_HEADER_SIZE + FREE_SPACE_HEADER_SIZE].copy_from_slice(free_space_header.as_bytes());   
    }

    pub fn get_page_header(&self) -> Result<PageHeader> {
        match PageHeader::try_read_from_bytes(&self.data[0..PAGE_HEADER_SIZE]) {
            Ok(page_header) => Ok(page_header),
            Err(e) => Err(anyhow::anyhow!("Failed to read page header: {}", e)),
        }
    }

    pub fn set_page_header(&mut self, page_header: PageHeader) {
        let page_header_bytes = page_header.as_bytes();
        self.data[0..PAGE_HEADER_SIZE].copy_from_slice(page_header_bytes);
    }

    pub fn get_next_free_space(&self) -> FreeSpaceHeader {
        let page_header = self.get_page_header().expect("Failed to read page header");
        let offset = page_header.next_free_space_offset as usize;
        let free_space_header = FreeSpaceHeader::read_from_bytes(&self.data[offset..offset + FREE_SPACE_HEADER_SIZE]).expect("Buffer size should match FreeSpaceHeader layout perfectly");
        return free_space_header
    }

    pub fn traverse_page(&self) -> Vec<(RowHeader, Vec<FieldData>)> {
        let mut rows = Vec::new();
        let page_header = self.get_page_header().expect("Failed to read page header");
        let mut row_pointer_offset = page_header.next_row_pointer_offset as usize;
        while row_pointer_offset != 0 {
            let row_pointer = RowPointer::read_from_bytes(&self.data[row_pointer_offset..row_pointer_offset + ROW_POINTER_SIZE]).expect("Buffer size should match RowPointer layout perfectly");
            let row_header = self.get_row_header(row_pointer.row_offset as usize);
            let row_data = self.get_row_data(row_pointer.row_offset, row_header.length);
            rows.push((row_header, row_data));
            row_pointer_offset = row_pointer.next_row_pointer_offset as usize;
        }   
        rows     
    }


    // Row management
    pub fn get_row_pointer(&self, row_index: usize) -> RowPointer {
        let row_pointer_offset = PAGE_HEADER_SIZE + (row_index * ROW_POINTER_SIZE);
        RowPointer::read_from_bytes(&self.data[row_pointer_offset..row_pointer_offset + ROW_POINTER_SIZE]).expect("Buffer size should match RowPointer layout perfectly")
    }

    pub fn get_row_header(&self, row_offset: usize) -> RowHeader {
        RowHeader::read_from_bytes(&self.data[row_offset..row_offset + ROW_HEADER_SIZE]).expect("Buffer size should match RowHeader layout perfectly")
    }

    pub fn set_row_header(&mut self, row_offset: usize, row_header: RowHeader) {
        let row_header_bytes = row_header.as_bytes();
        self.data[row_offset..row_offset + ROW_HEADER_SIZE].copy_from_slice(row_header_bytes);
    }

    pub fn get_row_data(&self, row_offset: u16, total_len: u16) -> Vec<FieldData> {
        let mut offset = row_offset as usize;
        let mut remaining_len = total_len as usize;
        let mut row_data: Vec<FieldData> = Vec::new();

        while remaining_len > 0 {
            let field_length = if cfg!(target_endian = "little") {
                u16::from_le_bytes(self.data[offset..offset + FIELD_LENGTH_SIZE].try_into().expect("Buffer size should match Field Length layout perfectly")) as usize
            } else {
                let field_length_bytes = &self.data[offset..offset + FIELD_LENGTH_SIZE];
                u16::from_be_bytes(field_length_bytes.try_into().expect("Buffer size should match Field Length layout perfectly")) as usize
            };

            let field_data = self.data[offset + FIELD_LENGTH_SIZE..offset + FIELD_LENGTH_SIZE + field_length].to_vec();
            row_data.push(FieldData { length: field_length as u16, data: field_data });
            offset += FIELD_LENGTH_SIZE + field_length;
            remaining_len -= FIELD_LENGTH_SIZE + field_length;
        }

        row_data
    }

    // Free Space Management
    pub fn get_free_space_header(&self, offset: usize) -> FreeSpaceHeader {
        FreeSpaceHeader::read_from_bytes(&self.data[offset..offset + FREE_SPACE_HEADER_SIZE]).expect("Buffer size should match FreeSpaceHeader layout perfectly")
    }

    // // Row DDL (Insert, Update, Delete) operations
    // pub fn insert_row(&mut self, row_data: Vec<FieldData>, txn_id: u32) -> Result<(), String> {
    //     let row_data_size = row_data.iter().map(|field| FIELD_LENGTH_SIZE + field.length as usize).sum::<usize>();
    //     let total_row_size = ROW_HEADER_SIZE + row_data_size;

    //     let page_header = self.get_page_header().expect("Failed to read page header");
       
    //     let page_total_free_space = page_header.free_space_total as usize;

    //     if total_row_size > page_total_free_space {
    //         return Err("Not enough space in the page to insert the row".to_string());
    //     }

    //     let mut next_free_space_offset = page_header.next_free_space_offset as usize;
    //     let mut prev_free_space_offset = 0;

    //     while next_free_space_offset != 0 {
    //         let free_space_header = self.get_free_space_header(next_free_space_offset);

    //         if free_space_header.free_space_size as usize == total_row_size {
    //             if next_free_space_offset == page_header.next_row_pointer_offset as usize {

    //             }
    //         }

    //         prev_free_space_offset = next_free_space_offset;
    //         next_free_space_offset = free_space_header.next_free_space_offset as usize;
    //     }

    //     return Err("Not enough space in the page to insert the row".to_string());
    // }
}