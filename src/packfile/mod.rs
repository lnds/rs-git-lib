pub mod refs;
use std::io::{Error, ErrorKind, Read, Result as IOResult};
use byteorder::{BigEndian, ReadBytesExt};

pub struct PackFileParser {
    packfile_data: Vec<u8>,
    lines: usize,
    version: u32,
    count_objects: usize,
}

pub const MAGIC_HEADER: u32 = 1_346_454_347; // "PACK"

impl PackFileParser {
    pub fn new() -> Self {
        PackFileParser {
            packfile_data: Vec::new(),
            lines: 0,
            version: 0,
            count_objects: 0,
        }
    }

    pub(crate) fn add_line(&mut self, line: &[u8]) -> IOResult<()> {
        match line[0] {
            1 => {
                self.lines += 1;
                self.packfile_data.extend_from_slice(&line[1..]);
                if self.lines == 1 {
                    let mut data: &[u8] = &line[1..];
                    let magic = data.read_u32::<BigEndian>()?;
                    if magic != MAGIC_HEADER {
                        return Err(Error::new(ErrorKind::Other, "Magic Header Not Found"));
                    }
                    self.version = data.read_u32::<BigEndian>()?;
                    self.count_objects = data.read_u32::<BigEndian>()? as usize;
                }
            }
            2 => {
                self.print_remote_message(std::str::from_utf8(&line[1..]).unwrap());
            }
            3 => {
                self.print_remote_error(std::str::from_utf8(&line[1..]).unwrap());
            }
            _ => return Err(Error::new(ErrorKind::Other, "Git server returned error")),
        }
        Ok(())
    }

    fn print_remote_message(&self, msg: &str) {
        println!("{}", msg);
    }

    fn print_remote_error(&self, msg: &str) {
        println!("{}", msg);
    }

    pub fn count_objects(&self) -> usize{
        self.count_objects
    }

}
