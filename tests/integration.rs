extern crate alloc;
#[cfg(test)]
#[path = "integration/contracts.rs"]
mod contracts;
#[cfg(test)]
#[path = "integration/file_delivery.rs"]
mod file_delivery;
#[cfg(test)]
#[path = "integration/fixture.rs"]
mod fixture;
#[cfg(test)]
#[path = "integration/reader_stream.rs"]
mod reader_stream;
#[cfg(test)]
#[path = "integration/selection.rs"]
mod selection;
