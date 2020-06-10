use nom::character::complete::space1;
use nom::IResult;
use rustc_serialize::hex::ToHex;
use std::str::{self, from_utf8, FromStr};
use std::vec::Vec;

#[derive(Debug)]
pub struct Tree {
    pub entries: Vec<TreeEntry>,
}

#[derive(Debug)]
pub struct TreeEntry {
    pub mode: EntryMode,
    pub path: String,
    pub sha: String,
}

#[derive(Debug, Clone)]
pub enum EntryMode {
    Normal,
    Executable,
    Symlink,
    Gitlink,
    SubDirectory,
}

impl Tree {
    pub fn parse(content: &[u8]) -> Option<Self> {
        if let IResult::Ok((_, entries)) = parse_tree_entries(content) {
            Some(Tree { entries })
        } else {
            None
        }
    }
}

impl FromStr for EntryMode {
    type Err = u8;
    fn from_str(mode: &str) -> Result<Self, Self::Err> {
        match mode {
            "100644" | "644" => Ok(EntryMode::Normal),
            "100755" | "755" => Ok(EntryMode::Executable),
            "120000" => Ok(EntryMode::Symlink),
            "160000" => Ok(EntryMode::Gitlink),
            "40000" => Ok(EntryMode::SubDirectory),
            _ => panic!("Unsupported file mode: {}", mode),
        }
    }
}

named!(parse_tree_entry(&[u8]) -> TreeEntry,
    do_parse!(
        mode: map_res!(take_until!(" "), from_utf8)  >>
        space1  >>
        path: map_res!(take_until!("\0"), from_utf8) >>
        take!(1) >>
        sha: take!(20) >>
        (
        TreeEntry {
            mode: EntryMode::from_str(mode).unwrap(),
            path: path.to_string(),
            sha: sha.to_hex(),
        }
        )
    )
);

fn parse_tree_entries(input: &[u8]) -> IResult<&[u8], Vec<TreeEntry>> {
    let mut result = vec![];
    let (mut input, elem) = parse_tree_entry(input)?;
    result.push(elem);
    if !input.is_empty() {
        loop {
            let tup = parse_tree_entry(input)?;
            input = tup.0;
            result.push(tup.1);
            if input.is_empty() {
                break;
            }
        }
    }
    Ok((input, result))
}

#[test]
fn test_parse_tree() {
    // The raw contents of a tree object.
    let input = [
        49, 48, 48, 54, 52, 52, 32, 46, 103, 105, 116, 105, 103, 110, 111, 114, 101, 0, 79, 255,
        178, 248, 156, 189, 143, 33, 105, 206, 153, 20, 189, 22, 189, 67, 120, 91, 179, 104, 49,
        48, 48, 54, 52, 52, 32, 67, 97, 114, 103, 111, 46, 116, 111, 109, 108, 0, 226, 11, 220, 57,
        33, 62, 223, 169, 46, 80, 98, 15, 155, 24, 209, 88, 234, 228, 138, 99, 49, 48, 48, 54, 52,
        52, 32, 82, 69, 65, 68, 77, 69, 46, 109, 100, 0, 189, 6, 31, 50, 207, 237, 81, 181, 168,
        222, 145, 109, 134, 186, 137, 235, 159, 208, 104, 242, 52, 48, 48, 48, 48, 32, 115, 114,
        99, 0, 44, 153, 32, 248, 175, 44, 114, 130, 179, 183, 191, 144, 34, 196, 7, 92, 15, 177,
        105, 86,
    ];
    if let IResult::Ok((_, _)) = parse_tree_entries(&input) {
        ()
    } else {
        panic!("Failed to parse tree");
    }
}
