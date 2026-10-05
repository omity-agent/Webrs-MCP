extern crate alloc;
#[cfg(test)]
#[path = "commandline/diagnostics.rs"]
mod diagnostics;
#[cfg(test)]
#[path = "commandline/grammar.rs"]
mod grammar;
#[cfg(test)]
#[path = "commandline/harness.rs"]
mod harness;
#[cfg(test)]
#[path = "commandline/materialization.rs"]
mod materialization;
#[cfg(test)]
#[path = "commandline/publishing.rs"]
mod publishing;
#[cfg(test)]
#[path = "commandline/querying.rs"]
mod querying;
#[cfg(test)]
#[path = "commandline/retrieval.rs"]
mod retrieval;
