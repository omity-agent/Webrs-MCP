pub(super) struct PseudoXml {
    text: String,
}
impl PseudoXml {
    pub(super) const fn new() -> Self {
        Self {
            text: String::new(),
        }
    }
    pub(super) fn open(&mut self, tag: &str) {
        self.text.push('<');
        self.text.push_str(tag);
        self.text.push_str(">\n");
    }
    pub(super) fn close(&mut self, tag: &str) {
        self.text.push_str("</");
        self.text.push_str(tag);
        self.text.push_str(">\n");
    }
    pub(super) fn text(&mut self, tag: &str, value: &str) {
        self.text.push('<');
        self.text.push_str(tag);
        self.text.push_str(">\n");
        self.text.push_str(value);
        self.text.push_str("\n</");
        self.text.push_str(tag);
        self.text.push_str(">\n");
    }
    pub(super) fn empty(&mut self, tag: &str) {
        self.text.push('<');
        self.text.push_str(tag);
        self.text.push_str(">\n\n</");
        self.text.push_str(tag);
        self.text.push_str(">\n");
    }
    pub(super) fn number(&mut self, tag: &str, value: usize) {
        self.text(tag, &value.to_string());
    }
    pub(super) fn indexed<T>(&mut self, values: &[T], render: impl Fn(&mut Self, &T)) {
        for (index, value) in values.iter().enumerate() {
            let tag = index.to_string();
            self.open(&tag);
            render(self, value);
            self.close(&tag);
        }
    }
    pub(super) fn finish(mut self) -> String {
        if let Some(last) = self.text.pop() {
            assert_eq!(last, '\n', "pseudo XML must end with a line feed");
        }
        self.text
    }
}
