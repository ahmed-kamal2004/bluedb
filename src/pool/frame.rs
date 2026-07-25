use anyhow::{Result};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, TryFromBytes};
use crate::engine::txn::IsolLvl;
use crate::engine::txn::txn::tk_row;

use super::{
    PAGE_SIZE, PAGE_HEADER_SIZE, FREE_SPACE_HEADER_SIZE, ROW_POINTER_SIZE, ROW_HEADER_SIZE, FIELD_LENGTH_SIZE,
};

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
///  Note: page number = offset / PAGE_SIZE
/// 
/// Free Space Header (4 bytes):
///         [Next Free Space Offset (2 bytes)]
///         [Free Space Size (2 bytes)]
/// 
/// Row Pointer (2 bytes):
///      [Row Offset (2 bytes)]
/// 
/// Row header layout (10 bytes):
///      [Tmin (4 bytes)][Tmax (4 bytes)][Length (2 bytes)] + [Row Data (variable length)]
/// 
/// Row Data layout:
///      [Field Length (2 bytes)][Field Data (variable length)][Field Length (2 bytes)][Field Data (variable length)]...
/// 
/// TODO: enable HOT using cid

#[repr(u8)]
#[derive(Immutable, TryFromBytes, KnownLayout, IntoBytes)]

pub enum FrameType {
    Normal = 0,
    TableMetadata = 1,
    Index = 2,
    IndexLeaf = 3,
}

#[repr(C, packed)]
#[derive(Immutable, TryFromBytes, KnownLayout, IntoBytes)]
pub struct PageHeader {
    pub tp: FrameType,
    pub free_space_total: u16,
    pub next_free_space_offset: u16,
    pub next_row_pointer_offset: u16,
    pub last_transaction_id: u32,
}

#[repr(C, packed)]
#[derive(Immutable, FromBytes, KnownLayout, IntoBytes, Debug)]
pub struct FreeSpaceHeader {
    pub next_free_space_offset: u16,
    pub free_space_size: u16,
}

#[repr(C, packed)]
#[derive(Immutable, FromBytes, KnownLayout, IntoBytes, Debug)]
pub struct RowPointer {
    pub row_offset: u16,
}

#[repr(C, packed)]
#[derive(Immutable, FromBytes, KnownLayout, IntoBytes, Debug)]
pub struct RowHeader {
    pub tmin: u32,
    pub tmax: u32,
    pub length: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldData {
    pub length: u16,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub data: [u8; PAGE_SIZE],
}

impl Frame {
    // Page initialization and header management
    pub fn initialize(&mut self, frame_type: FrameType, txn_id: u32) {

        // Add the page header
        let page_header = PageHeader {
            tp: frame_type,
            free_space_total: (PAGE_SIZE - PAGE_HEADER_SIZE) as u16,
            next_free_space_offset: PAGE_HEADER_SIZE as u16,
            next_row_pointer_offset: 0,
            last_transaction_id: txn_id,
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

    pub fn get_next_free_space(&self) -> Result<FreeSpaceHeader> {
        let page_header = self.get_page_header()?;
        let offset = page_header.next_free_space_offset as usize;
        match FreeSpaceHeader::read_from_bytes(&self.data[offset..offset + FREE_SPACE_HEADER_SIZE]) {
            Ok(free_space_header) => Ok(free_space_header),
            Err(e) => Err(anyhow::anyhow!("Failed to read free space header: {}", e)),
        }
    }

    pub fn traverse_page(&self) -> Result<Vec<(RowHeader, Vec<FieldData>)>> {
        let mut rows = Vec::new();
        let page_header = self.get_page_header()?;
        let mut row_pointer_offset = page_header.next_row_pointer_offset as usize;
        while row_pointer_offset >= PAGE_HEADER_SIZE {
            let row_pointer = RowPointer::read_from_bytes(&self.data[row_pointer_offset..row_pointer_offset + ROW_POINTER_SIZE]).expect("Buffer size should match RowPointer layout perfectly");
            let row_header = self.get_row_header(row_pointer.row_offset as usize);
            let row_data = self.get_row_data(row_pointer.row_offset, row_header.length)?;
            rows.push((row_header, row_data));
            row_pointer_offset -= ROW_POINTER_SIZE;
        }
        Ok(rows)
    }

    pub fn retrieve_active_rows(&self, current_txn: u32, isol_lvl: IsolLvl) -> Result<Vec<Vec<FieldData>>> {
        let mut rows = Vec::new();
        let page_header = self.get_page_header()?;
        let mut row_pointer_offset = page_header.next_row_pointer_offset as usize;
        while row_pointer_offset >= PAGE_HEADER_SIZE {
            let row_pointer = RowPointer::read_from_bytes(&self.data[row_pointer_offset..row_pointer_offset + ROW_POINTER_SIZE]).expect("Buffer size should match RowPointer layout perfectly");
            let row_header = self.get_row_header(row_pointer.row_offset as usize);
            let row_data = self.get_row_data(row_pointer.row_offset, row_header.length)?;
            if tk_row(current_txn, isol_lvl, row_header.tmin, row_header.tmax) {
                rows.push(row_data);
            }
            row_pointer_offset -= ROW_POINTER_SIZE;
        }
        Ok(rows)
    }

    // Row management
    pub fn get_row_pointer(&self, row_index: usize) -> RowPointer {
        let row_pointer_offset = PAGE_HEADER_SIZE + (row_index * ROW_POINTER_SIZE);
        RowPointer::read_from_bytes(&self.data[row_pointer_offset..row_pointer_offset + ROW_POINTER_SIZE]).expect("Buffer size should match RowPointer layout perfectly")
    }

    pub fn get_row_pointer_by_offset(&self, row_pointer_offset: usize) -> RowPointer {
        RowPointer::read_from_bytes(&self.data[row_pointer_offset..row_pointer_offset + ROW_POINTER_SIZE]).expect("Buffer size should match RowPointer layout perfectly")
    }

    pub fn get_row_header(&self, row_offset: usize) -> RowHeader {
        RowHeader::read_from_bytes(&self.data[row_offset..row_offset + ROW_HEADER_SIZE]).expect("Buffer size should match RowHeader layout perfectly")
    }

    pub fn set_row_header(&mut self, row_offset: usize, row_header: RowHeader) {
        let row_header_bytes = row_header.as_bytes();
        self.data[row_offset..row_offset + ROW_HEADER_SIZE].copy_from_slice(row_header_bytes);
    }

    pub fn get_row_data(&self, row_offset: u16, total_len: u16) -> Result<Vec<FieldData>> {
        let mut offset = row_offset as usize + ROW_HEADER_SIZE;
        let mut remaining_len = total_len as usize;
        let mut row_data: Vec<FieldData> = Vec::new();

        while remaining_len > 0 {
            let field_length = if cfg!(target_endian = "little") {
                u16::from_le_bytes(self.data[offset..offset + FIELD_LENGTH_SIZE].try_into()?) as usize
            } else {
                let field_length_bytes = &self.data[offset..offset + FIELD_LENGTH_SIZE];
                u16::from_be_bytes(field_length_bytes.try_into()?) as usize
            };

            let field_data = self.data[offset + FIELD_LENGTH_SIZE..offset + FIELD_LENGTH_SIZE + field_length].to_vec();
            row_data.push(FieldData { length: field_length as u16, data: field_data });
            offset += FIELD_LENGTH_SIZE + field_length;
            remaining_len -= FIELD_LENGTH_SIZE + field_length;
        }

        Ok(row_data)
    }

    // Free Space Management
    pub fn get_free_space_header(&self, offset: usize) -> FreeSpaceHeader {
        FreeSpaceHeader::read_from_bytes(&self.data[offset..offset + FREE_SPACE_HEADER_SIZE]).expect("Buffer size should match FreeSpaceHeader layout perfectly")
    }

    // Row DDL (Insert, Update, Delete) operations
    pub fn insert_row(&mut self, row_data: Vec<FieldData>, txn_id: u32) -> Result<u16> {
        let row_data_size = row_data.iter().map(|field| FIELD_LENGTH_SIZE + field.length as usize).sum::<usize>();
        let total_row_size = ROW_HEADER_SIZE + row_data_size;

        let page_header = self.get_page_header().expect("Failed to read page header");
       
        let page_total_free_space = page_header.free_space_total as usize;

        if total_row_size > page_total_free_space {
            return Err(anyhow::anyhow!("Not enough space in the page to insert the row"));
        }

        let mut next_free_space_offset = page_header.next_free_space_offset as usize;
        let first_free_space_offset = next_free_space_offset;

        while next_free_space_offset != 0 {
            let free_space_header = self.get_free_space_header(next_free_space_offset);

            if free_space_header.free_space_size as usize == total_row_size {
                if next_free_space_offset == first_free_space_offset {
                    next_free_space_offset = free_space_header.next_free_space_offset as usize;
                    continue;
                } else {
                    // Write the row header
                    let row_header = RowHeader {
                        tmin: txn_id,
                        tmax: 0,
                        length: row_data_size as u16,
                    };

                    let row_header_bytes = row_header.as_bytes();
                    self.data[next_free_space_offset..next_free_space_offset + ROW_HEADER_SIZE].copy_from_slice(&row_header_bytes);

                    // Write the row data
                    let mut data_offset = next_free_space_offset + ROW_HEADER_SIZE;
                    for field in &row_data {
                        let field_length_bytes = if cfg!(target_endian = "little") {
                            field.length.to_le_bytes()
                        } else {
                            field.length.to_be_bytes()
                        };
                        self.data[data_offset..data_offset + FIELD_LENGTH_SIZE].copy_from_slice(&field_length_bytes);
                        data_offset += FIELD_LENGTH_SIZE;
                        self.data[data_offset..data_offset + field.length as usize].copy_from_slice(&field.data);
                        data_offset += field.length as usize;
                    }

                    // first let's move the existing free space header, so we can add the row pointer at the end of the page.
                    let mut first_free_space_header = self.get_free_space_header(first_free_space_offset);
                    first_free_space_header.free_space_size -= ROW_POINTER_SIZE as u16;
                    let first_free_space_header_bytes = first_free_space_header.as_bytes();
                    let new_free_space_offset = first_free_space_offset + ROW_POINTER_SIZE;
                    self.data[new_free_space_offset..new_free_space_offset + FREE_SPACE_HEADER_SIZE].copy_from_slice(&first_free_space_header_bytes);

                    
                    // Add the row pointer
                    let row_pointer = RowPointer {
                        row_offset: next_free_space_offset as u16,
                    };
                    let row_pointer_bytes = row_pointer.as_bytes();
                    self.data[first_free_space_offset ..first_free_space_offset + ROW_POINTER_SIZE].copy_from_slice(&row_pointer_bytes);

                    // Update the page header
                    let mut updated_page_header = page_header;
                    updated_page_header.free_space_total -= total_row_size as u16;
                    updated_page_header.free_space_total -= ROW_POINTER_SIZE as u16;

                    updated_page_header.next_free_space_offset = new_free_space_offset as u16;
                    updated_page_header.next_row_pointer_offset = first_free_space_offset as u16;
                    self.set_page_header(updated_page_header);

                    return Ok(next_free_space_offset as u16);
                }
            } else if free_space_header.free_space_size as usize > total_row_size {
                if next_free_space_offset == first_free_space_offset && ((free_space_header.free_space_size as usize) < (FREE_SPACE_HEADER_SIZE + ROW_POINTER_SIZE + total_row_size)) {
                    next_free_space_offset = free_space_header.next_free_space_offset as usize;
                    continue;
                }
                // Write the row header
                let row_header = RowHeader {
                    tmin: txn_id,
                    tmax: 0,
                    length: row_data_size as u16,
                };

                let row_header_bytes = row_header.as_bytes();
                let final_offset = next_free_space_offset + free_space_header.free_space_size as usize - total_row_size;
                self.data[final_offset..final_offset + ROW_HEADER_SIZE].copy_from_slice(&row_header_bytes);

                // Write the row data
                let mut data_offset = final_offset + ROW_HEADER_SIZE;
                for field in &row_data {
                    let field_length_bytes = if cfg!(target_endian = "little") {
                        field.length.to_le_bytes()
                    } else {
                        field.length.to_be_bytes()
                    };
                    self.data[data_offset..data_offset + FIELD_LENGTH_SIZE].copy_from_slice(&field_length_bytes);
                    data_offset += FIELD_LENGTH_SIZE;
                    self.data[data_offset..data_offset + field.length as usize].copy_from_slice(&field.data);
                    data_offset += field.length as usize;
                }

                // Update the free space header
                let mut updated_free_space_header = free_space_header;
                updated_free_space_header.free_space_size -= total_row_size as u16;
                let updated_free_space_header_bytes = updated_free_space_header.as_bytes();
                self.data[next_free_space_offset..next_free_space_offset + FREE_SPACE_HEADER_SIZE].copy_from_slice(&updated_free_space_header_bytes);


                // Add the row pointer
                let row_pointer = RowPointer {
                    row_offset: final_offset as u16,
                };
                let row_pointer_bytes = row_pointer.as_bytes();

                let first_free_space_header = self.get_free_space_header(first_free_space_offset);
                self.data[first_free_space_offset..first_free_space_offset + ROW_POINTER_SIZE].copy_from_slice(&row_pointer_bytes);

                // Update the first free space header
                let mut updated_first_free_space_header = first_free_space_header;
                updated_first_free_space_header.free_space_size -= ROW_POINTER_SIZE as u16;
                let updated_first_free_space_header_bytes = updated_first_free_space_header.as_bytes();
                self.data[first_free_space_offset + ROW_POINTER_SIZE..first_free_space_offset + FREE_SPACE_HEADER_SIZE + ROW_POINTER_SIZE].copy_from_slice(updated_first_free_space_header_bytes);

                // Update the page header
                let mut updated_page_header = page_header;
                updated_page_header.free_space_total -= total_row_size as u16;
                updated_page_header.free_space_total -= ROW_POINTER_SIZE as u16;
                updated_page_header.next_free_space_offset = (first_free_space_offset + ROW_POINTER_SIZE) as u16;
                updated_page_header.next_row_pointer_offset = first_free_space_offset as u16;
                self.set_page_header(updated_page_header);

                return Ok(final_offset as u16);
            } 
            next_free_space_offset = free_space_header.next_free_space_offset as usize;
        }
        return Err(anyhow::anyhow!("Not enough space in the page to insert the row"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_frame() -> Frame {
        Frame {
            data: [0; PAGE_SIZE],
        }
    }

    fn field(data: &[u8]) -> FieldData {
        FieldData {
            length: data.len() as u16,
            data: data.to_vec(),
        }
    }

    fn row_payload_len(fields: &[FieldData]) -> usize {
        fields
            .iter()
            .map(|f| FIELD_LENGTH_SIZE + f.length as usize)
            .sum::<usize>()
    }

    #[test]
    fn initialize_sets_expected_page_and_free_space_headers() -> Result<()> {
        let mut frame = new_frame();
        frame.initialize(FrameType::Normal, 7);

        let header = frame.get_page_header().expect("page header should be readable");
        let free_space_total = header.free_space_total;
        let next_free_space_offset = header.next_free_space_offset;
        let next_row_pointer_offset = header.next_row_pointer_offset;
        let last_transaction_id = header.last_transaction_id;

        assert_eq!(free_space_total, (PAGE_SIZE - PAGE_HEADER_SIZE) as u16);
        assert_eq!(next_free_space_offset, PAGE_HEADER_SIZE as u16);
        assert_eq!(next_row_pointer_offset, 0);
        assert_eq!(last_transaction_id, 7);

        let fs = frame.get_next_free_space()?;
        let fs_next_offset = fs.next_free_space_offset;
        let fs_size = fs.free_space_size;
        assert_eq!(fs_next_offset, 0);
        assert_eq!(fs_size, (PAGE_SIZE - PAGE_HEADER_SIZE) as u16);
        Ok(())
    }

    #[test]
    fn insert_row_and_traverse_page_round_trips_data() -> Result<()> {
        let mut frame = new_frame();
        frame.initialize(FrameType::Normal, 100);

        let row = vec![field(b"abc"), field(b"de")];
        frame.insert_row(row, 99).expect("insert should succeed");

        let rows = frame.traverse_page()?;
        for (ri, (row_header, fields)) in rows.iter().enumerate() {
            let dbg_tmin = row_header.tmin;
            let dbg_tmax = row_header.tmax;
            let dbg_len = row_header.length;
            println!(
                "Row {}: tmin={}, tmax={}, length={}",
                ri, dbg_tmin, dbg_tmax, dbg_len
            );
            for (fi, field) in fields.iter().enumerate() {
                println!("  Field {}: length={}, data={:?}", fi, field.length, field.data);
            }
        }

        let page_header = frame.get_page_header().expect("page header should be readable");
        let free_space_total = page_header.free_space_total;
        let next_free_space_offset = page_header.next_free_space_offset;
        let next_row_pointer_offset = page_header.next_row_pointer_offset;
        println!(
            "Page Header: free_space_total={}, next_free_space_offset={}, next_row_pointer_offset={}",
            free_space_total, next_free_space_offset, next_row_pointer_offset
        );

        println!("Sum of used space: {}", (PAGE_HEADER_SIZE + ROW_POINTER_SIZE + ROW_HEADER_SIZE + 5 + 2 * FIELD_LENGTH_SIZE) as usize);

        assert_eq!(rows.len(), 1);

        let (row_header, fields) = &rows[0];
        let tmin = row_header.tmin;
        let tmax = row_header.tmax;
        let length = row_header.length;
        assert_eq!(tmin, 99);
        assert_eq!(tmax, 0);
        assert_eq!(length, 9);

        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].length, 3);
        assert_eq!(fields[0].data, b"abc");
        assert_eq!(fields[1].length, 2);
        assert_eq!(fields[1].data, b"de");

        let header = frame.get_page_header().expect("page header should be readable");
        let free_space_total = header.free_space_total;
        assert_eq!(free_space_total, ((PAGE_SIZE - PAGE_HEADER_SIZE) - (ROW_HEADER_SIZE + 9) - ROW_POINTER_SIZE) as u16);

        Ok(())
    }

    #[test]
    fn insert_row_returns_error_when_page_has_not_enough_space() {
        let mut frame = new_frame();
        frame.initialize(FrameType::Normal, 100);

        let too_large_payload = vec![0_u8; PAGE_SIZE];
        let row = vec![field(too_large_payload.as_slice())];

        let result = frame.insert_row(row, 1);
        assert!(result.is_err());
    }

    #[test]
    fn insert_then_read_same_row_by_returned_offset() -> Result<()> {
        let mut frame = new_frame();
        frame.initialize(FrameType::Normal, 10);

        let row = vec![field(b"first"), field(b"row")];
        let inserted_offset = frame
            .insert_row(row, 123)
            .expect("insert should return row offset");

        let row_header = frame.get_row_header(inserted_offset as usize);
        let tmin = row_header.tmin;
        let tmax = row_header.tmax;
        assert_eq!(tmin, 123);
        assert_eq!(tmax, 0);

        let fields = frame.get_row_data(inserted_offset, row_header.length)?;
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].data, b"first");
        assert_eq!(fields[1].data, b"row");
        Ok(())
    }

    #[test]
    fn traverse_page_returns_all_inserted_rows_in_pointer_order() -> Result<()> {
        let mut frame = new_frame();
        frame.initialize(FrameType::Normal, 50);

        let row1 = vec![field(b"r1")];
        let row2 = vec![field(b"r2a"), field(b"r2b")];
        let row3 = vec![field(b"r3")];

        frame.insert_row(row1, 1).expect("row1 insert should succeed");
        frame.insert_row(row2, 2).expect("row2 insert should succeed");
        frame.insert_row(row3, 3).expect("row3 insert should succeed");

        let rows = frame.traverse_page()?;
        assert_eq!(rows.len(), 3);

        let row0_tmin = rows[0].0.tmin;
        assert_eq!(row0_tmin, 3);
        assert_eq!(rows[0].1[0].data, b"r3");

        let row1_tmin = rows[1].0.tmin;
        assert_eq!(row1_tmin, 2);
        assert_eq!(rows[1].1.len(), 2);
        assert_eq!(rows[1].1[0].data, b"r2a");
        assert_eq!(rows[1].1[1].data, b"r2b");

        let row2_tmin = rows[2].0.tmin;
        assert_eq!(row2_tmin, 1);
        assert_eq!(rows[2].1[0].data, b"r1");
        Ok(())
    }

    #[test]
    fn page_header_and_free_space_are_updated_after_each_insert() {
        let mut frame = new_frame();
        frame.initialize(FrameType::Normal, 77);

        let initial_header = frame.get_page_header().expect("header should be readable");
        let initial_free = initial_header.free_space_total as usize;

        let row1 = vec![field(b"aa")];
        let row1_payload = row_payload_len(&row1);
        frame.insert_row(row1, 101).expect("row1 insert should succeed");

        let header_after_first = frame.get_page_header().expect("header should be readable");
        let first_last_txn = header_after_first.last_transaction_id;
        let first_next_row_ptr = header_after_first.next_row_pointer_offset;
        assert_eq!(first_last_txn, 77);
        assert_eq!(first_next_row_ptr, PAGE_HEADER_SIZE as u16);

        let expected_after_first = initial_free - (ROW_HEADER_SIZE + row1_payload + ROW_POINTER_SIZE);
        assert_eq!(header_after_first.free_space_total as usize, expected_after_first);

        let row2 = vec![field(b"bbbb"), field(b"cc")];
        let row2_payload = row_payload_len(&row2);
        frame.insert_row(row2, 102).expect("row2 insert should succeed");

        let header_after_second = frame.get_page_header().expect("header should be readable");
        let expected_after_second = expected_after_first - (ROW_HEADER_SIZE + row2_payload + ROW_POINTER_SIZE);
        assert_eq!(header_after_second.free_space_total as usize, expected_after_second);
        let second_next_row_ptr = header_after_second.next_row_pointer_offset;
        let second_next_free_ptr = header_after_second.next_free_space_offset;
        assert_eq!(second_next_row_ptr, (PAGE_HEADER_SIZE + ROW_POINTER_SIZE) as u16);
        assert_eq!(second_next_free_ptr, (PAGE_HEADER_SIZE + 2 * ROW_POINTER_SIZE) as u16);
    }

    #[test]
    fn insert_until_full_then_traverse_and_print_output() -> Result<()> {
        let mut frame = new_frame();
        frame.initialize(FrameType::Normal, 500);

        let mut inserted = 0usize;
        let mut next_txn = 1u32;

        loop {
            let row = vec![field(b"xx")];
            match frame.insert_row(row, next_txn) {
                Ok(_) => {
                    inserted += 1;
                    next_txn += 1;
                }
                Err(err) => {
                    println!("Stopped inserting after {} rows: {}", inserted, err);
                    break;
                }
            }
        }

        let rows = frame.traverse_page()?;
        println!("Traversed rows count: {}", rows.len());

        for (ri, (row_header, fields)) in rows.iter().enumerate().take(5) {
            let tmin = row_header.tmin;
            let tmax = row_header.tmax;
            let len = row_header.length;
            println!("Row {} => tmin={}, tmax={}, length={}", ri, tmin, tmax, len);
            for (fi, f) in fields.iter().enumerate() {
                println!("  Field {} => length={}, data={:?}", fi, f.length, f.data);
            }
        }

        let header = frame.get_page_header().expect("page header should be readable");
        let free_space_total = header.free_space_total;
        let next_free_space_offset = header.next_free_space_offset;
        let next_row_pointer_offset = header.next_row_pointer_offset;
        let last_inserted_ptr = frame.get_row_pointer_by_offset(next_row_pointer_offset as usize);
        let first_free_space_header = frame.get_free_space_header(next_free_space_offset as usize);
        println!(
            "Final header => free_space_total={}, next_free_space_offset={}, next_row_pointer_offset={}, last_inserted_ptr={:?}, first_free_space_header={:?}",
            free_space_total, next_free_space_offset, next_row_pointer_offset, last_inserted_ptr, first_free_space_header
        );

        assert!(inserted > 0, "at least one row should be inserted");
        println!("Total rows traversed: {}, total inserted: {}", rows.len(), inserted);
        assert_eq!(rows.len(), inserted, "traversal should return all inserted rows");
        Ok(())
    }
}