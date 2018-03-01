//! # stayalive
//!
//! [![crates.io](https://img.shields.io/crates/v/stayalive.svg)](https://crates.io/crates/stayalive)
//! [![docs.rs](https://docs.rs/stayalive/badge.svg)](https://docs.rs/stayalive)
//! [![downloads](https://img.shields.io/crates/d/stayalive.svg)](https://crates.io/crates/stayalive)
//! [![Build Status](https://travis-ci.org/chridou/stayalive.svg?branch=master)](https://travis-ci.org/chridou/stayalive)
//! [![license-mit](http://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/chridou/stayalive/blob/master/LICENSE-MIT)
//! [![license-apache](http://img.shields.io/badge/license-APACHE-blue.svg)](https://github.com/chridou/stayalive/blob/master/LICENSE-APACHE)
//!
//! A collection of resilience patterns.
//!
//! **Still in a very early phase**
//!
//! ## Usage
//!
//! Get the latest version for your `Cargo.toml` from
//! [crates.io](https://crates.io/crates/stayalive).
//!
//! Add this crate root:
//!
//! ```rust
//! extern crate stayalive;
//! ```
#[macro_use]
extern crate failure;
extern crate threadpool;

pub mod circuitbreakers;
pub mod bulkheads;
