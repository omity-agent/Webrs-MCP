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
#[path = "integration/network/redirects.rs"]
mod redirects;
#[cfg(test)]
#[path = "integration/network/response_budget.rs"]
mod response_budget;
#[cfg(test)]
#[path = "integration/scheduling.rs"]
mod scheduling;
#[cfg(test)]
#[path = "integration/selection.rs"]
mod selection;
#[cfg(test)]
#[path = "integration/network/service_limits.rs"]
mod service_limits;
