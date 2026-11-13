fn dangling(input: &str) -> Option<&[String]> {
    Some([String::from(input)].as_slice())
}
