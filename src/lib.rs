#[doc = include_str!("../README.md")]
#[allow(clippy::all)]
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(deref_nullptr)]
#[allow(improper_ctypes)]
#[allow(dead_code)]
mod bindings;
/// TODO
pub mod callback;
/// A debug macro used throughout the crate
pub mod connection;
/// A module containing all possible errors produced by the library
pub mod error;
/// Creates connections and manages high level interfaces for iOS devices
pub mod idevice;
/// A bare bones representation of a service running on a device.
/// Useful for services that don't have modules or for running raw commands
pub mod service;
/// A module that contains all abstractions for built-in services
pub mod services;

mod rustls_extern;
