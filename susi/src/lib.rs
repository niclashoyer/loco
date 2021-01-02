#![cfg_attr(not(feature = "std"), no_std)]

pub mod message;
pub mod receiver;
pub mod sender;

#[cfg(test)]
pub mod tests_mock;

/// Errors returned from a SUSI receiver or sender
#[derive(Debug, PartialEq)]
pub enum Error {
	IOError,
	TimerError,
}
