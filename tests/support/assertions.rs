use rmcp::serde_json::Value;
pub(crate) trait JsonProbe {
    fn at(&self, path: &str) -> &Value;
    fn text(&self, path: &str) -> &str;
    fn list(&self, path: &str) -> &[Value];
}
impl JsonProbe for Value {
    fn at(&self, path: &str) -> &Value {
        self.pointer(path)
            .unwrap_or_else(|| panic!("missing {path}: {self}"))
    }
    fn text(&self, path: &str) -> &str {
        self.at(path)
            .as_str()
            .unwrap_or_else(|| panic!("expected string at {path}: {self}"))
    }
    fn list(&self, path: &str) -> &[Value] {
        self.at(path)
            .as_array()
            .unwrap_or_else(|| panic!("expected array at {path}: {self}"))
    }
}
