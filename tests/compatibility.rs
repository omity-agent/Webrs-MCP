extern crate alloc;
#[cfg(test)]
#[path = "compatibility/exchange.rs"]
mod exchange;
#[cfg(test)]
#[path = "compatibility/guardrails.rs"]
mod guardrails;
#[cfg(test)]
#[path = "compatibility/invocation.rs"]
mod invocation;
#[cfg(test)]
#[path = "compatibility/persisted.rs"]
mod persisted;
#[cfg(test)]
#[path = "compatibility/pipe.rs"]
mod pipe;
#[cfg(test)]
#[path = "compatibility/wire.rs"]
mod wire;
