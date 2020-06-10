use crate::delta;
use crate::store::commit::Commit;
use crate::store::tree::Tree;
use crate::utils::sha1_hash_hex;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::cell::RefCell;
use std::fs;
use std::fs::File;
use std::io::{Read, Result as IOResult, Write};
use std::path::PathBuf;

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

    pub fn patch(&self, patch: &[u8]) -> Self {
        GitObject {
            object_type: self.object_type,
            content: delta::patch(&self.content, &patch),
            sha: RefCell::new(None),
        }
    }

    ///
    /// Opens the given object from loose form in the repo.
    ///
    pub fn open(repo: &str, sha1: &str) -> IOResult<Self> {
        println!("open (repo={}, sha1={})", repo, sha1);
        let path = object_path(repo, sha1);
        println!("file = {:?}", path);
        let mut inflated = Vec::new();
        let file = File::open(path)?;
        let mut z = ZlibDecoder::new(file);
        z.read_to_end(&mut inflated)?;
        // .expect("Error inflating object");

        let sha1_checksum = sha1_hash_hex(&inflated);
        assert_eq!(sha1_checksum, sha1);

        let split_idx = inflated.iter().position(|x| *x == 0).unwrap();
        let (object_type, size) = {
            let header = std::str::from_utf8(&inflated[..split_idx]).unwrap();
            GitObject::parse_header(header)
        };

        let mut footer = Vec::new();
        footer.extend_from_slice(&inflated[split_idx + 1..]);

        assert_eq!(footer.len(), size);

        Ok(GitObject {
            object_type,
            content: footer,
            sha: RefCell::new(Some(sha1.to_owned())),
        })
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

    fn parse_header(header: &str) -> (GitObjectType, usize) {
        let split: Vec<&str> = header.split(' ').collect();
        if split.len() == 2 {
            let (t, s) = (split[0], split[1]);
            let obj_type = match t {
                "commit" => GitObjectType::Commit,
                "tree" => GitObjectType::Tree,
                "blob" => GitObjectType::Blob,
                "tag" => GitObjectType::Tag,
                _ => panic!("unknown object type"),
            };
            let size = s.parse::<usize>().unwrap();

            (obj_type, size)
        } else {
            panic!("Bad object header")
        }
    }

    ///
    /// Parses the internal representation of this object into a Commit.
    /// Returns `None` if the object is not a Commit.
    ///
    pub fn as_commit(&self) -> Option<Commit> {
        if let GitObjectType::Commit = self.object_type {
            Commit::from_raw(&self)
        } else {
            None
        }
    }

    ///
    /// Parses the internal representation of this object into a Tree.
    /// Returns `None` if the object is not a Tree.
    ///
    pub fn as_tree(&self) -> Option<Tree> {
        if let GitObjectType::Tree = self.object_type {
            Tree::parse(&self.content)
        } else {
            None
        }
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
