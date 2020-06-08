use std::cell::RefCell;

#[derive(Debug, Copy, Clone, FromPrimitive)]
pub enum GitObjectType {
    Commit = 1,
    Tree = 2,
    Blob = 3,
    Tag = 4,
}

#[derive(Debug, Clone)]
pub struct GitObject {
    pub object_type: GitObjectType,
    pub content: Vec<u8>,
    sha: RefCell<Option<String>>,
}



impl GitObject {
    pub fn new(object_type: GitObjectType, content: Vec<u8>) -> Self {
        GitObject {
            object_type,
            content,
            sha: RefCell::new(None),
        }
    }
}