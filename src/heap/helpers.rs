use super::super::constants::{
    FIELD_LENGTH_SIZE, FREE_SPACE_HEADER_SIZE, PAGE_HEADER_SIZE, ROW_HEADER_SIZE, ROW_POINTER_SIZE,
};
use crate::heap::heap::{FieldData, Row, RowHeader, RowPointer};
use crate::heap::heap::{FreeSpaceHeader, HeapPageHeader, HeapPageView};
use anyhow::Result;
use zerocopy::IntoBytes;
use zerocopy::TryFromBytes;

impl HeapPageView<'_> {
    /* Header management */
    pub fn get_header(&self) -> Result<HeapPageHeader> {
        match HeapPageHeader::try_read_from_bytes(&self.page.data[0..PAGE_HEADER_SIZE]) {
            Ok(pg_hdr) => Ok(pg_hdr),
            Err(e) => Err(anyhow::anyhow!("Failed to read page header: {}", e)),
        }
    }

    pub fn set_header(&mut self, hdr: &HeapPageHeader) {
        let pg_hdr_bytes = hdr.as_bytes();
        self.page.data[0..PAGE_HEADER_SIZE].copy_from_slice(pg_hdr_bytes);
    }

    /* Row management */
    pub fn read_row(&self, rw_ptr_offset: usize) -> Result<Row> {
        let rw_ptr = self.get_rw_ptr_by_ofst(rw_ptr_offset)?;

        let rw_ofst = rw_ptr.rw_ofst as usize;

        let rw_hdr = self.get_rw_hdr(rw_ofst)?;

        let rw_dta = self.get_rw_dta(rw_ofst + ROW_HEADER_SIZE, rw_hdr.len)?;
        Ok(Row {
            header: rw_hdr,
            fields: rw_dta,
        })
    }

    /// Returns the row pointer at the given index.
    pub fn get_rw_ptr_by_idx(&self, rw_idx: usize) -> Result<RowPointer> {
        let rw_ptr_offset = rw_idx * ROW_POINTER_SIZE + PAGE_HEADER_SIZE;
        RowPointer::try_read_from_bytes(
            &self.page.data[rw_ptr_offset..rw_ptr_offset + ROW_POINTER_SIZE],
        )
        .map_err(|e| anyhow::anyhow!("Failed to read row pointer: {}", e))
    }

    /// Returns the row pointer at the given offset.
    pub fn get_rw_ptr_by_ofst(&self, rw_ptr_offset: usize) -> Result<RowPointer> {
        RowPointer::try_read_from_bytes(
            &self.page.data[rw_ptr_offset..rw_ptr_offset + ROW_POINTER_SIZE],
        )
        .map_err(|e| anyhow::anyhow!("Failed to read row pointer: {}", e))
    }

    /// Returns the row header at the given offset.
    pub fn get_rw_hdr(&self, rw_offset: usize) -> Result<RowHeader> {
        RowHeader::try_read_from_bytes(&self.page.data[rw_offset..rw_offset + ROW_HEADER_SIZE])
            .map_err(|e| anyhow::anyhow!("Failed to read row header: {}", e))
    }

    /// Returns the row data at the given offset and length.
    pub fn get_rw_dta(&self, mut offset: usize, len: u16) -> Result<Vec<FieldData>> {
        let mut rw_dt = Vec::new();
        let mut remaining_len = len;

        while remaining_len > 0 {
            let field_length = if cfg!(target_endian = "little") {
                u16::from_le_bytes(self.page.data[offset..offset + FIELD_LENGTH_SIZE].try_into()?)
                    as usize
            } else {
                let field_length_bytes = &self.page.data[offset..offset + FIELD_LENGTH_SIZE];
                u16::from_be_bytes(field_length_bytes.try_into()?) as usize
            };

            let field_data = self.page.data
                [offset + FIELD_LENGTH_SIZE..offset + FIELD_LENGTH_SIZE + field_length]
                .to_vec();
            rw_dt.push(FieldData {
                len: field_length as u16,
                data: field_data,
            });
            offset += FIELD_LENGTH_SIZE + field_length;
            remaining_len -= (FIELD_LENGTH_SIZE + field_length) as u16;
        }

        Ok(rw_dt)
    }

    /* Free Space Header (Getter -- Setter) */
    pub fn get_fr_spc_hdr(&self, offset: usize) -> Result<FreeSpaceHeader> {
        FreeSpaceHeader::try_read_from_bytes(
            &self.page.data[offset..offset + FREE_SPACE_HEADER_SIZE],
        )
        .map_err(|e| anyhow::anyhow!("Failed to read free space header: {}", e))
    }
}
