//! Service Framework (SF) for Nintendo Switch
//!
//! This crate implements the **Service Framework** - the IPC serialization layer
//! used by Horizon OS services. The name "SF" comes from the CMIF protocol's
//! magic headers: `"SFCI"` (Service Framework Command Input) and `"SFCO"`
//! (Service Framework Command Output).
//!
//! # Architecture
//!
//! The IPC stack on Horizon OS is layered:
//!
//! ```text
//! ┌─────────────────────────────┐
//! │  Service APIs (fs, sm, etc) │  Application layer
//! ├─────────────────────────────┤
//! │  CMIF / TIPC                │  Command serialization (SF layer)
//! ├─────────────────────────────┤
//! │  HIPC                       │  Message framing & descriptors
//! ├─────────────────────────────┤
//! │  Kernel SVCs                │  Transport (SendSyncRequest, etc)
//! └─────────────────────────────┘
//! ```
//!
//! This crate provides the middle layers (HIPC, and CMIF/TIPC),
//! enabling Rust code to communicate with system services.
//!
//! # Protocols
//!
//! - **HIPC**: Low-level message format handling buffer descriptors, handles,
//!   and raw data layout. See the [`hipc`] module for details.
//! - **CMIF**: Command interface with domain support (object multiplexing).
//!   Uses `"SFCI"`/`"SFCO"` magic headers. See the [`cmif`] module for details.
//! - **TIPC**: Simplified protocol introduced in HOS 12.0.0. No domains,
//!   command ID stored in HIPC message type. See the [`tipc`] module for details.

#![no_std]

extern crate nx_panic_handler; // Provides #[panic_handler]

pub mod cmif;
pub mod hipc;
pub mod service;
mod service_name;
pub mod tipc;

pub use service_name::ServiceName;

#[cfg(feature = "ffi")]
pub mod ffi;
