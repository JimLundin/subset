#[derive(Serialize, Deserialize)]
struct Flattened {
    #[serde(flatten)]
    value: Inner,
}
