impl<'de> Deserialize<'de> for Seq {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Seq, D::Error> {
        u64::deserialize(d).map(Seq)
    }
}
