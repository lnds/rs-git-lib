use std::cell::RefCell;
use std::fs::File;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::fs;
use std::io::{Read, Write, Result as IOResult};
use std::path::PathBuf;
use crate::utils::sha1_hash_hex;

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

    #[allow(unused)]
    pub fn write(&self, repo: &str) -> IOResult<()> {
        let (sha1, blob) = self.encode();
        let path = object_path(repo, &sha1);

        fs::create_dir_all(path.parent().unwrap())?;

        let file = File::create(&path)?;
        let mut z = ZlibEncoder::new(file, Compression::default());
        z.write_all(&blob[..])?;
        Ok(())
    }

    ///
    /// Encodes the object into packed format, returning the
    /// SHA and encoded representation.
    ///
    pub fn encode(&self) -> (String, Vec<u8>) {
        // encoding:
        // header ++ content
        let mut encoded = self.header();
        encoded.extend_from_slice(&self.content);
        (sha1_hash_hex(&encoded[..]), encoded)
    }

    pub fn sha(&self) -> String {
        {
            let mut cache = self.sha.borrow_mut();
            if cache.is_some() {
                return cache.as_ref().unwrap().clone();
            }
            let (hash, _) = self.encode();
            *cache = Some(hash);
        }
        self.sha()
    }

    fn header(&self) -> Vec<u8> {
        // header:
        // "type size \0"
        let str_type = match self.object_type {
            GitObjectType::Commit => "commit",
            GitObjectType::Tree => "tree",
            GitObjectType::Blob => "blob",
            GitObjectType::Tag => "tag",
        };
        let str_size = self.content.len().to_string();
        let res: String = [str_type, " ", &str_size[..], "\0"].concat();
        res.into_bytes()
    }
}


fn object_path(repo: &str, sha: &str) -> PathBuf {
    let mut path = PathBuf::new();
    path.push(repo);
    path.push(".git");
    path.push("objects");
    path.push(&sha[..2]);
    path.push(&sha[2..40]);
    path
}
