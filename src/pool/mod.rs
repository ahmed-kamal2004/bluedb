pub mod pool;
pub mod frame;

// Page-wide constants
const PAGE_SIZE: usize = 8192;

// Page header layout constants
const PAGE_HEADER_SIZE: usize = 11;
const PAGE_TYPE_SIZE: usize = 1;
const FREE_SPACE_TOTAL_SIZE: usize = 2;
const NEXT_FREE_SPACE_OFFSET_SIZE: usize = 2;
const NEXT_ROW_POINTER_OFFSET_SIZE: usize = 2;
const LAST_TRANSACTION_ID_SIZE: usize = 4;

// Free space header layout constants
const FREE_SPACE_HEADER_SIZE: usize = 4;
// const NEXT_FREE_SPACE_OFFSET_SIZE: usize = 2; // declared above

// Row pointer layout constants
const ROW_POINTER_SIZE: usize = 2;

// Row header layout constants
const ROW_HEADER_SIZE: usize = 10;
const ROW_TMIN_SIZE: usize = 4;
const ROW_TMAX_SIZE: usize = 4;
const ROW_DATA_LENGTH_SIZE: usize = 2;

// Row data layout constants
const FIELD_LENGTH_SIZE: usize = 2;