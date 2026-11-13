struct Pair([u64; 2]);

const BACKOFF: [Duration; 2] = [Duration::from_secs(1), Duration::from_secs(5)];
