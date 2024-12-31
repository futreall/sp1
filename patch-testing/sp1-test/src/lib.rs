/// How many items to generate for the corpus.
pub const DEFAULT_CORPUS_COUNT: u8 = 100;

/// The maximum length of an item in the corpus, if applicable.
pub const DEFAULT_CORPUS_MAX_LEN: usize = 100;

pub static SERIAL_LOCK: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

pub fn lock_serial() -> parking_lot::MutexGuard<'static, ()> {
    SERIAL_LOCK.lock()
}

/// Append common edge cases to the corpus.
///
/// Like all 0s or all 1s or the empty string.
pub fn add_hash_fn_edge_cases(corpus: &mut Vec<Vec<u8>>) {
    let max_len = DEFAULT_CORPUS_COUNT;
    corpus.push(vec![]);

    // push inputs of all 0s
    for len in 1..=max_len {
        corpus.push(vec![0; len as usize]);
    }

    // push inputs of all 255s
    for len in 1..=max_len {
        corpus.push(vec![255; len as usize]);
    }
}

/// Generate `count` random preimages with bounded length `len`.
pub fn random_preimages_with_bounded_len(count: u8, len: usize) -> Vec<Vec<u8>> {
    use rand::distributions::Distribution;

    (0..count)
        .map(|_| {
            let len =
                rand::distributions::Uniform::new(0_usize, len).sample(&mut rand::thread_rng());

            (0..len).map(|_| rand::random::<u8>()).collect::<Vec<u8>>()
        })
        .collect()
}

pub fn random_prehash() -> [u8; 32] {
    use sha2_v0_9_8::Digest;

    let prehash = rand::random::<[u8; 32]>();

    let mut sha = sha2_v0_9_8::Sha256::new();
    sha.update(prehash);

    sha.finalize().into()
}

#[doc(inline)]
pub use sp1_test_macro::sp1_test;
