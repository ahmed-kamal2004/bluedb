use super::super::constants::ROW_HEADER_SIZE;
use crate::constants::FIELD_LENGTH_SIZE;
use crate::constants::FREE_SPACE_HEADER_SIZE;
use crate::constants::ROW_POINTER_SIZE;
use crate::heap::heap::FieldData;
use crate::heap::heap::HeapPageView;
use crate::heap::heap::RowHeader;
use crate::heap::heap::RowPointer;
use anyhow::Result;
use zerocopy::IntoBytes;

pub trait Insert {
    fn insert_row(&mut self, row: &Vec<FieldData>, txn_id: u32) -> Result<()>;
}

impl Insert for HeapPageView<'_> {
    fn insert_row(&mut self, row: &Vec<FieldData>, txn_id: u32) -> Result<()> {
        // Data size
        let rw_sz = row
            .iter()
            .map(|f| f.len as usize + FIELD_LENGTH_SIZE)
            .sum::<usize>();

        // Total row size (header + data)
        let fl_rw_sz = rw_sz + ROW_HEADER_SIZE;

        // Initial check for available space
        let pg_hdr = self.get_header()?;
        let pg_tl_fr_spc = pg_hdr.fr_spc_tl as usize;

        if fl_rw_sz + ROW_POINTER_SIZE >= pg_tl_fr_spc {
            return Err(anyhow::anyhow!("Not enough space to insert row"));
        }

        // Ok, then now we can proceed to the next detailed check for row insertion, which includes checking the free space headers and finding a suitable location for the new row.
        let mut nxt_fr_spc_ofst = pg_hdr.nxt_fr_spc_ofst as usize;
        let mut prv_fr_spc_ofst = 0;
        let frst_fr_spc_ofst = nxt_fr_spc_ofst;

        let mut frst_fr_spc_hdr = self.get_fr_spc_hdr(frst_fr_spc_ofst)?;
        if frst_fr_spc_hdr.fr_spc_sz as usize >= ROW_POINTER_SIZE + FREE_SPACE_HEADER_SIZE {
            frst_fr_spc_hdr.fr_spc_sz -= ROW_POINTER_SIZE as u16;
        } else {
            return Err(anyhow::anyhow!("Not enough space to insert row"));
        }

        loop {
            if nxt_fr_spc_ofst == 0 {
                return Err(anyhow::anyhow!(
                    "No suitable free space found for row insertion"
                ));
            }

            let mut fr_spc_hdr = self.get_fr_spc_hdr(nxt_fr_spc_ofst)?;

            if fr_spc_hdr.fr_spc_sz as usize == fl_rw_sz {
                if nxt_fr_spc_ofst == frst_fr_spc_ofst {
                    // This is the first free space header, I won't be able to insert to it, as I will need to take from it a space of ROW_POINTER_SIZE + fl_rw_sz, which will be larger than the free space size, so I will need to move to the next free space header.
                    prv_fr_spc_ofst = nxt_fr_spc_ofst;
                    nxt_fr_spc_ofst = fr_spc_hdr.nxt_fr_spc_ofst as usize;
                    continue;
                }

                // We have found a free space header that has exactly the size we need for the new row, so we can insert the row here.
                // We will insert the row at the offset of the free space header, and then we will need to remove this free space header from the list of free space headers, as it will no longer be available after the insertion.
                // We need to update the previous free space header to point to the next free space header, effectively removing the current free space header from the list.
                // We need also to update the page header to reflect the new total free space size.

                // first step, move the first space header by the size of the row offset.
                let nw_frst_fr_spc_ofst = frst_fr_spc_ofst + ROW_POINTER_SIZE;

                // second step, update the page header to reflect the new total free space size.
                let mut pg_hdr = self.get_header()?;
                pg_hdr.fr_spc_tl -= fl_rw_sz as u16;
                pg_hdr.fr_spc_tl -= ROW_POINTER_SIZE as u16;

                pg_hdr.nxt_fr_spc_ofst = nw_frst_fr_spc_ofst as u16;

                // third step, update the prv_fr_spc_ofst to point to the nxt_fr_spc_ofst, effectively removing the current free space header from the list.
                if prv_fr_spc_ofst == 0 {
                    // we need to update the page header itself.
                    // unreachable, as the first free space header is always the first one in the list, and we have already handled that case above.
                    return Err(anyhow::anyhow!("Unexpected state: prv_fr_spc_ofst is 0"));
                } else if prv_fr_spc_ofst == frst_fr_spc_ofst {
                    // we need to update the first free space header to point to the next free space header.
                    frst_fr_spc_hdr.nxt_fr_spc_ofst = fr_spc_hdr.nxt_fr_spc_ofst;
                } else {
                    // otherwise.
                    let mut prv_fr_spc_hdr = self.get_fr_spc_hdr(prv_fr_spc_ofst)?;
                    prv_fr_spc_hdr.nxt_fr_spc_ofst = fr_spc_hdr.nxt_fr_spc_ofst;

                    // update the previous free space header in the page data.
                    let prv_fr_spc_hdr_bytes = prv_fr_spc_hdr.as_bytes();
                    self.page.data[prv_fr_spc_ofst..prv_fr_spc_ofst + prv_fr_spc_hdr_bytes.len()]
                        .copy_from_slice(&prv_fr_spc_hdr_bytes);
                }

                // changing the page data will be the first changing step.
                // -> insert the row at the specific offset.
                self.insrt_rw_at_ofst(nxt_fr_spc_ofst, row.clone(), txn_id)?;

                // -> insert the row pointer at the specific offset.
                let rw_ptr = RowPointer {
                    rw_ofst: nxt_fr_spc_ofst as u16,
                };
                let rw_ptr_bytes = rw_ptr.as_bytes();
                self.page.data[frst_fr_spc_ofst..frst_fr_spc_ofst + rw_ptr_bytes.len()]
                    .copy_from_slice(&rw_ptr_bytes);

                // -> update the first free space header place.
                let frst_fr_spc_hdr_bytes = frst_fr_spc_hdr.as_bytes();
                self.page.data
                    [nw_frst_fr_spc_ofst..nw_frst_fr_spc_ofst + frst_fr_spc_hdr_bytes.len()]
                    .copy_from_slice(&frst_fr_spc_hdr_bytes);
                // -> update the page header.
                pg_hdr.nxt_rw_ptr_ofst = frst_fr_spc_ofst as u16;
                self.set_header(&pg_hdr);

                return Ok(());
            } else if fr_spc_hdr.fr_spc_sz as usize > fl_rw_sz {
                if nxt_fr_spc_ofst == frst_fr_spc_ofst {
                    // In this case, we are trying to insert to the first free space.
                    // We need to check if it can contain at least the size of the row + the size of the row pointer + the size of the free space header, otherwise we won't be able to insert.
                    if frst_fr_spc_hdr.fr_spc_sz as usize
                        >= fl_rw_sz + ROW_POINTER_SIZE + FREE_SPACE_HEADER_SIZE
                    {
                        // We can insert here in the first free space.
                        // Four things to do:
                        // . Insert the row at the (offset + size - fl_rw_size) and update the size of the free space.
                        // . Insert the row pointer at the offset and update the size of the free space.
                        // . Update the free space header to reflect the new size and offset.
                        // . Update the page header.

                        // first step, move the first space header by the size of the row offset.
                        frst_fr_spc_hdr.fr_spc_sz -= (fl_rw_sz) as u16;

                        let mut pg_hdr = self.get_header()?;
                        pg_hdr.fr_spc_tl -= fl_rw_sz as u16;
                        pg_hdr.fr_spc_tl -= ROW_POINTER_SIZE as u16;

                        pg_hdr.nxt_fr_spc_ofst = (nxt_fr_spc_ofst + ROW_POINTER_SIZE) as u16;

                        // insert the row.
                        let insrt_ofst =
                            nxt_fr_spc_ofst + frst_fr_spc_hdr.fr_spc_sz as usize + ROW_POINTER_SIZE;
                        self.insrt_rw_at_ofst(insrt_ofst, row.clone(), txn_id)?;

                        // insert the row pointer.
                        let rw_ptr = RowPointer {
                            rw_ofst: insrt_ofst as u16,
                        };
                        let rw_ptr_bytes = rw_ptr.as_bytes();
                        self.page.data[frst_fr_spc_ofst..frst_fr_spc_ofst + ROW_POINTER_SIZE]
                            .copy_from_slice(&rw_ptr_bytes);

                        // Update the free space header to reflect the new size and offset.
                        let fr_spc_hdr_bytes = frst_fr_spc_hdr.as_bytes();
                        self.page.data[frst_fr_spc_ofst + ROW_POINTER_SIZE
                            ..frst_fr_spc_ofst + ROW_POINTER_SIZE + FREE_SPACE_HEADER_SIZE]
                            .copy_from_slice(&fr_spc_hdr_bytes);

                        // Update the page header.
                        pg_hdr.nxt_rw_ptr_ofst = frst_fr_spc_ofst as u16;
                        self.set_header(&pg_hdr);

                        return Ok(());
                    } else {
                        // Well, we just can't insert to this free space.
                        prv_fr_spc_ofst = nxt_fr_spc_ofst;
                        nxt_fr_spc_ofst = fr_spc_hdr.nxt_fr_spc_ofst as usize;
                        continue;
                    }
                } else {
                    // In this case, there is a space, and this isn't the first free space.
                    if fr_spc_hdr.fr_spc_sz >= (fl_rw_sz + FREE_SPACE_HEADER_SIZE) as u16 {
                        // first step, decrease the size of the free space header by the size of the row.
                        fr_spc_hdr.fr_spc_sz -= fl_rw_sz as u16;

                        // update the page header to reflect the new total free space size.
                        let mut pg_hdr = self.get_header()?;
                        pg_hdr.fr_spc_tl -= fl_rw_sz as u16;
                        pg_hdr.fr_spc_tl -= ROW_POINTER_SIZE as u16;

                        pg_hdr.nxt_fr_spc_ofst = (frst_fr_spc_ofst + ROW_POINTER_SIZE) as u16;

                        // insert the row.
                        let insrt_ofst = nxt_fr_spc_ofst + fr_spc_hdr.fr_spc_sz as usize;
                        self.insrt_rw_at_ofst(insrt_ofst, row.clone(), txn_id)?;

                        // insert the row pointer.
                        let rw_ptr = RowPointer {
                            rw_ofst: insrt_ofst as u16,
                        };
                        let rw_ptr_bytes = rw_ptr.as_bytes();
                        self.page.data[frst_fr_spc_ofst..frst_fr_spc_ofst + ROW_POINTER_SIZE]
                            .copy_from_slice(&rw_ptr_bytes);

                        // Update the free space header to reflect the new size and offset.
                        let fr_spc_hdr_bytes: &[u8] = fr_spc_hdr.as_bytes();
                        self.page.data[nxt_fr_spc_ofst..nxt_fr_spc_ofst + FREE_SPACE_HEADER_SIZE]
                            .copy_from_slice(&fr_spc_hdr_bytes);

                        // update the first free space header to point to the next free space header.
                        let frst_fr_spc_hdr_bytes = frst_fr_spc_hdr.as_bytes();
                        self.page.data[frst_fr_spc_ofst + ROW_POINTER_SIZE
                            ..frst_fr_spc_ofst + ROW_POINTER_SIZE + FREE_SPACE_HEADER_SIZE]
                            .copy_from_slice(&frst_fr_spc_hdr_bytes);

                        // Update the page header.
                        pg_hdr.nxt_rw_ptr_ofst = frst_fr_spc_ofst as u16;
                        self.set_header(&pg_hdr);

                        return Ok(());
                    } else {
                        prv_fr_spc_ofst = nxt_fr_spc_ofst;
                        nxt_fr_spc_ofst = fr_spc_hdr.nxt_fr_spc_ofst as usize;
                        continue;
                    }
                }
            } else {
                prv_fr_spc_ofst = nxt_fr_spc_ofst;
                nxt_fr_spc_ofst = fr_spc_hdr.nxt_fr_spc_ofst as usize;
                continue;
            }
        }
    }
}

impl HeapPageView<'_> {
    fn insrt_rw_at_ofst(&mut self, rw_ofst: usize, rw: Vec<FieldData>, txn_id: u32) -> Result<()> {
        let rw_sz = rw
            .iter()
            .map(|f| f.len as usize + FIELD_LENGTH_SIZE)
            .sum::<usize>();

        // Write the row header
        let rw_hdr = RowHeader {
            tmin: txn_id,
            tmax: 0,
            len: rw_sz as u16,
        };
        self.page.data[rw_ofst..rw_ofst + ROW_HEADER_SIZE].copy_from_slice(&rw_hdr.as_bytes());

        // Write the row data
        let mut ofst = rw_ofst + ROW_HEADER_SIZE;
        for field in rw {
            self.page.data[ofst..ofst + FIELD_LENGTH_SIZE]
                .copy_from_slice(&(field.len.to_le_bytes()));
            ofst += FIELD_LENGTH_SIZE;
            self.page.data[ofst..ofst + field.len as usize].copy_from_slice(&field.data);
            ofst += field.len as usize;
        }

        Ok(())
    }
}
