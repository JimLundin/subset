#[cfg(test)]
mod tests {
    struct Case {
        input: String,
        touch: Touch,
        name: String,
    }

    #[test]
    fn names() {
        let cases = Vec::from([
            Case { input: String::from("a"), touch: Touch::None, name: String::from("A") },
        ]);
        let pairs = Vec::from([(String::from("a"), String::from("A"))]);
        assert_eq!(cases.len(), pairs.len());
    }
}
