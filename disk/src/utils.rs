#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileMetaData {
    file_name: String,
    file_inode: usize,
    file_num_pages: usize,
    file_permissions: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DirectoryMetaData {
    files_metadata: Vec<FileMetaData>,
    dir_permissions: String,
}
