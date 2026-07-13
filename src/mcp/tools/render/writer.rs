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
    pub(super) fn finish(mut self) -> String {
        assert_eq!(
            self.text.pop(),
            Some('\n'),
            "pseudo XML must end with a line feed"
        );
        self.text
    }
}
