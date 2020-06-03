
#[derive(Debug, Copy, Clone)]
pub enum GitObjectType {
     Commit = 1,
     Tree = 2,
     Blob = 3,
     Tag = 4,
}

#[derive(Clone)]
pub struct GitObject {
    pub object_type: GitObjectType,
    pub content: Vec<u8>,
    sha: RefCell<Option<String>>,
}