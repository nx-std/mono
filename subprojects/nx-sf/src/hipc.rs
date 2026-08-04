//! HIPC (Horizon Inter-Process Communication) protocol implementation.
//!
//! HIPC is the low-level message serialization protocol for IPC on Nintendo
//! Switch's Horizon OS. It defines the wire format for passing data, handles,
//! and buffer descriptors between processes via kernel supervisor calls.
//!
//! # Protocol Stack
//!
//! HIPC is the transport layer in the Horizon IPC stack:
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │  Service APIs (fs, sm, hid, etc.)   │  Application layer
//! ├─────────────────────────────────────┤
//! │  CMIF or TIPC                       │  Command serialization
//! ├─────────────────────────────────────┤
//! │  HIPC  ← this module                │  Message framing & descriptors
//! ├─────────────────────────────────────┤
//! │  Kernel SVCs (SendSyncRequest, etc) │  Transport
//! └─────────────────────────────────────┘
//! ```
//!
//! Two command protocols build on HIPC:
//! - **CMIF** (Command Message Interface Format): Original protocol with domain
//!   support for multiplexing objects. Uses magic `"SFCI"`/`"SFCO"` headers.
//! - **TIPC** (Tiny IPC): Simplified protocol introduced in HOS 12.0.0. No
//!   domains, command ID stored directly in HIPC message type field.
//!
//! # Message Location
//!
//! Messages are written to the Thread Local Region (TLR) at offset 0x0. Each
//! thread has a 0x200-byte IPC buffer in its TLS area. The kernel reads request
//! messages from and writes response messages to this buffer.
//!
//! # Message Layout
//!
//! ```text
//! Offset  Size   Field
//! ──────────────────────────────────────────────────────────────
//! 0x00    0x08   Header (message type, descriptor counts)
//! 0x08    0x04   SpecialHeader (optional: PID flag, handle counts)
//! 0x0C    0x08   ProcessId (optional: if send_pid is set)
//!         var    Copy Handles (4 bytes × num_copy_handles)
//!         var    Move Handles (4 bytes × num_move_handles)
//!         var    Send Statics / Type X (8 bytes each)
//!         var    Send Buffers / Type A (12 bytes each)
//!         var    Recv Buffers / Type B (12 bytes each)
//!         var    Exch Buffers / Type W (12 bytes each)
//!         var    Data Words (raw payload, 4 bytes each)
//!         var    Recv List / Type C (8 bytes each)
//! ──────────────────────────────────────────────────────────────
//! ```
//!
//! # Descriptor Types (Switchbrew Naming)
//!
//! HIPC defines several descriptor types for transferring data:
//!
//! | Type | Name          | Direction      | Mechanism        | Size Limit |
//! |------|---------------|----------------|------------------|------------|
//! | X    | Send Static   | Client→Server  | Pointer (copy)   | 64 KB      |
//! | A    | Send Buffer   | Client→Server  | Memory mapping   | 4 GB       |
//! | B    | Recv Buffer   | Server→Client  | Memory mapping   | 4 GB       |
//! | W    | Exch Buffer   | Bidirectional  | Memory mapping   | 4 GB       |
//! | C    | Recv List     | Server→Client  | Pointer (copy)   | 64 KB      |
//!
//! ## Pointer Descriptors (Type X / Send Static)
//!
//! Used for small data transfers. The kernel copies data between process
//! address spaces. Each descriptor has a 6-bit index for matching send/receive
//! pairs. Maximum transfer size is 64 KB (16-bit size field).
//!
//! ## Buffer Descriptors (Types A/B/W)
//!
//! Used for larger data transfers via memory mapping:
//!
//! - **Send (A)**: Client memory mapped read-only (R--) into server
//! - **Recv (B)**: Client memory mapped read-write (RW-) into server
//! - **Exchange (W)**: Same buffer for both directions (RW-)
//!
//! Memory mappings are automatically released when the kernel processes the
//! reply message. Buffer descriptors support sizes up to 4 GB (36-bit size).
//!
//! ## Receive List (Type C)
//!
//! Pre-allocated client buffers for receiving pointer data. The server writes
//! to these using send statics. The `recv_static_mode` header field controls:
//! - Mode 0: No receive list
//! - Mode 2: Auto-calculate count from send statics
//! - Mode 2+n: Exactly n receive list entries
//!
//! # Handle Passing
//!
//! Kernel handles (sessions, events, shared memory, etc.) can be passed:
//!
//! - **Copy Handle**: The kernel duplicates the handle. Both processes retain
//!   independent references to the same kernel object.
//! - **Move Handle**: Ownership transfers to the receiver. The sender's handle
//!   becomes invalid after the call.
//!
//! # Address Encoding
//!
//! 64-bit addresses are split across bitfields to fit the packed descriptor
//! format. The encoding varies by descriptor type:
//!
//! **Static Descriptor (8 bytes):**
//! ```text
//! Bits 0-5:   index (6 bits)
//! Bits 6-11:  address[36:41] (6 bits)
//! Bits 12-15: address[32:35] (4 bits)
//! Bits 16-31: size (16 bits, max 64KB)
//! Bits 32-63: address[0:31] (32 bits)
//! ```
//!
//! **Buffer Descriptor (12 bytes):**
//! ```text
//! Bits 0-31:  size[0:31] (32 bits)
//! Bits 32-63: address[0:31] (32 bits)
//! Bits 64-65: mode (2 bits)
//! Bits 66-87: address[36:57] (22 bits)
//! Bits 88-91: size[32:35] (4 bits)
//! Bits 92-95: address[32:35] (4 bits)
//! ```
//!
//! # Server-side request parsing
//!
//! Only the **client** side of HIPC is implemented in this crate: requests are
//! built (see [`make_request`] / [`Request`]) and responses are parsed (see
//! [`parse_response`] / [`Response`]). There is intentionally no
//! `parse_request` counterpart - `nx-sf` exists to talk *to* Horizon services
//! from a homebrew client, so the inbound request path the kernel would deliver
//! to a server is out of scope. If server hosting is added later it will land
//! in a dedicated module rather than here.
//!
//! # Design boundary: synchronous IPC only
//!
//! Buffers attached to a request are modeled as borrows ([`InputBuffer`],
//! [`OutputBuffer`], ...) held by the request value and released when its
//! consuming `send` returns. That model is valid **because** the IPC here is
//! synchronous: the kernel's access window over every descriptor target is
//! exactly the dynamic extent of `SendSyncRequest`, which the borrow region
//! covers. Asynchronous or user-buffer request SVCs keep using the memory
//! after the initiating call returns - a window no borrow can express. If
//! those are ever wrapped, they need owned-buffer designs (ownership
//! transferred in, handed back on completion), not this module's loans.
//!
//! # References
//!
//! - [Switchbrew IPC Marshalling](https://switchbrew.org/wiki/IPC_Marshalling)
//! - libnx `sf/hipc.h` (fincs, SciresM)

mod buffer;
mod request;
mod response;
mod wire;

pub(crate) use self::request::HipcRequestBuilder;
pub use self::{
    buffer::{
        InOutBuffer,
        InPointer,
        InputBuffer,
        OutPointer,
        OutputBuffer,
    },
    request::{
        HIPC_MAX_DESCRIPTORS,
        HIPC_MAX_RECV_LIST,
        HipcPayload,
        HipcRequest,
        SendError,
        WriteError,
    },
    response::{
        Envelope,
        Response,
        ResponseParseError,
        parse_response,
        parse_response_envelope,
    },
    // Wire descriptor types stay readable (their accessors document the
    // format), but their constructors are `pub(crate)`: descriptors erase
    // loans into raw addresses, so only the loan-collecting builders (CMIF,
    // TIPC) may produce them.
    wire::{
        BufferDescriptor,
        BufferMode,
        Header,
        MessageType,
        RecvListEntry,
        SpecialHeader,
        StaticDescriptor,
    },
};
