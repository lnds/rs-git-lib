pub fn sha1_hash_hex(input: &[u8]) -> String {
    use crypto::digest::Digest;
    use crypto::sha1::Sha1;

    let mut hasher = Sha1::new();
    hasher.input(input);

    hasher.result_str()
}