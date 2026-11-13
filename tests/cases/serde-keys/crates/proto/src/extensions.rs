#[derive(Serialize, Deserialize)]
struct Hello {
    #[serde(default)]
    host: String,
}
