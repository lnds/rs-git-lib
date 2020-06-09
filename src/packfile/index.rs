use crate::store::object::GitObject;
use rustc_serialize::hex::FromHex;

type SHA = [u8; 20];

static MAGIC: [u8; 4] = [255, 116, 79, 99];
static VERSION: u32 = 2;

///
/// Version 2 of the Git Packfile Index containing separate
/// tables for the offsets, fanouts, and shas.
///
/// see http://shafiul.github.io/gitbook/7_the_packfile.html
///
pub struct PackIndex {
    fanout: [u32; 256],
    offsets: Vec<u32>,
    shas: Vec<SHA>,
    checksums: Vec<u32>,
    pack_sha: String,
}


impl PackIndex {

    pub fn from_objects(
        mut objects: Vec<(usize, u32, GitObject)>,
        pack_sha: &str,
        dir: Option<&str>,
    ) -> Self {
        let size = objects.len();
        let mut fanout = [0u32; 256];
        let mut offsets = vec![0; size];
        let mut shas = vec![[0; 20]; size];
        let mut checksums: Vec<u32> = vec![0; size];

        // Sort the objects by SHA
        objects.sort_by(|&(_, _, ref oa), &(_, _, ref ob)| oa.sha().cmp(&ob.sha()));

        for (i, &(offset, crc, ref obj)) in objects.iter().enumerate() {
            if let Some(path) = dir {
                let _ = obj.write(path);
            }
            let mut sha = [0u8; 20];
            let vsha = &obj.sha().from_hex().unwrap();
            sha.clone_from_slice(&vsha);

            // Checksum should be of packed content in the packfile.
            let fanout_start = sha[0] as usize;
            // By definition of the fanout table we need to increment every entry >= this sha
            for f in fanout.iter_mut().skip(fanout_start) {
                *f += 1;
            }
            shas[i] = sha;
            offsets[i] = offset as u32;
            checksums[i] = crc;
        }
        assert_eq!(size as u32, fanout[255]);
        PackIndex {
            fanout,
            offsets,
            shas,
            checksums,
            pack_sha: pack_sha.to_string(),
        }
    }
}