pub fn sha1_hash_hex(input: &[u8]) -> String {
    use crypto::digest::Digest;
    use crypto::sha1::Sha1;

    let mut hasher = Sha1::new();
    hasher.input(input);

    hasher.result_str()
}

pub fn sha1_hash(input: &[u8]) -> Vec<u8> {
    use crypto::digest::Digest;
    use crypto::sha1::Sha1;

    let mut hasher = Sha1::new();
    hasher.input(input);
    let mut buf = vec![0; hasher.output_bytes()];
    hasher.result(&mut buf);
    buf
}

pub fn is_sha(id: &str) -> bool {
    id.len() == 40 && id.chars().all(|c| c.is_digit(16))
}
