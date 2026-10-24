extern crate alloc;
#[cfg(test)]
mod commandline;
#[path = "../support/envelope.rs"]
mod envelope;
#[cfg(test)]
mod hosting;
#[path = "../support/listener.rs"]
mod listener;
#[cfg(test)]
mod pagination;
#[path = "../support/process.rs"]
mod process;
#[cfg(test)]
mod stdio;
