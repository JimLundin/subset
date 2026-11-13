#[cfg(test)]
mod tests {
    #[test]
    fn names() {
        let cases = Vec::from([
            (String::from("a"), Touch::None, String::from("A")),
            (String::from("b"), Touch::Screen, String::from("B")),
        ]);
        assert_eq!(cases.len(), 2);
    }
}
