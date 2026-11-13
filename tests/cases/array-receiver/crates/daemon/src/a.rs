fn joined(first: String, second: String) -> String {
    [first, second].concat()
}

fn mapped() -> Vec<String> {
    ["a", "b"].map(String::from).to_vec()
}

fn owned(first: String, second: String) -> Vec<String> {
    [first, second].into_iter().collect()
}

fn iterated() -> Vec<String> {
    ["a", "b"].iter().copied().map(String::from).collect()
}

fn borrowed() -> usize {
    (&["a", "b"]).len()
}

fn grouped() -> usize {
    ((["a", "b"])).len()
}

fn grouped_argument() -> usize {
    Vec::from((["a", "b"])).len()
}

fn repeated() -> Vec<u8> {
    [0u8; 4].map(|value| value).to_vec()
}

const WORDS: &[&str] = &(["a", "b"]);

fn effects() {
    for value in ["a", "b"].into_iter() {
        consume(value);
    }
}

fn consume(_value: &str) {}
