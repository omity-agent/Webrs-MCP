use alloc::vec::IntoIter;
use core::ops::Deref;
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Warnings {
    values: Vec<String>,
}
impl Warnings {
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self { values: Vec::new() }
    }
    #[inline]
    pub fn push<Message>(&mut self, message: Message)
    where
        Message: Into<String>,
    {
        self.values.push(message.into());
    }
    #[inline]
    pub fn push_unique<Message>(&mut self, message: Message)
    where
        Message: Into<String>,
    {
        let owned_message = message.into();
        if !self
            .values
            .iter()
            .any(|existing| existing == &owned_message)
        {
            self.values.push(owned_message);
        }
    }
    #[inline]
    pub fn extend<Messages>(&mut self, messages: Messages)
    where
        Messages: IntoIterator<Item = String>,
    {
        self.values.extend(messages);
    }
    #[inline]
    pub fn append(&mut self, mut other: Self) {
        self.values.append(&mut other.values);
    }
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    #[inline]
    #[must_use]
    pub fn into_option(self) -> Option<Vec<String>> {
        (!self.is_empty()).then_some(self.values)
    }
    #[inline]
    #[must_use]
    pub fn join(&self, separator: &str) -> String {
        self.values.join(separator)
    }
}
impl Deref for Warnings {
    type Target = [String];
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}
impl PartialEq<Vec<String>> for Warnings {
    #[inline]
    fn eq(&self, other: &Vec<String>) -> bool {
        self.values == *other
    }
}
impl PartialEq<Warnings> for Vec<String> {
    #[inline]
    fn eq(&self, other: &Warnings) -> bool {
        *self == other.values
    }
}
impl IntoIterator for Warnings {
    type Item = String;
    type IntoIter = IntoIter<String>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}
