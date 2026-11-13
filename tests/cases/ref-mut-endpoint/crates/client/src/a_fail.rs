struct Transcript {
    items: Vec<Item>,
}

impl Transcript {
    fn push(&mut self, item: Item) {
        self.items.push(item);
    }
}
