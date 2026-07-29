/* FRAME - RELATED constants */
// Page-wide constants
pub const PAGE_SIZE: usize = 8192;

// Page header layout constants
pub const PAGE_HEADER_SIZE: usize = 11;
pub const PAGE_TYPE_SIZE: usize = 1;
pub const FREE_SPACE_TOTAL_SIZE: usize = 2;
pub const NEXT_FREE_SPACE_OFFSET_SIZE: usize = 2;
pub const NEXT_ROW_POINTER_OFFSET_SIZE: usize = 2;
pub const LAST_TRANSACTION_ID_SIZE: usize = 4;

// Free space header layout constants
pub const FREE_SPACE_HEADER_SIZE: usize = 4;
// const NEXT_FREE_SPACE_OFFSET_SIZE: usize = 2; // declared above

// Row pointer layout constants
pub const ROW_POINTER_SIZE: usize = 2;

// Row header layout constants
pub const ROW_HEADER_SIZE: usize = 10;
pub const ROW_TMIN_SIZE: usize = 4;
pub const ROW_TMAX_SIZE: usize = 4;
pub const ROW_DATA_LENGTH_SIZE: usize = 2;

// Row data layout constants
pub const FIELD_LENGTH_SIZE: usize = 2;

/* FILE - RELATED constants */
pub const FILE_UNIX_PAGE_SIZE: usize = 4096;

/* Naming */
pub const CATALOG_FILE_NAME: &str = "bluedb.catalog";
