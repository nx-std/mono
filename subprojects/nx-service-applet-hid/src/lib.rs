//! # nx-service-applet-hid
//!
//! The `controller` library applet: the system's controller UI, which pairs and
//! arranges the Joy-Cons and gamepads an application asks for, guides the user
//! through the wrist straps, updates controller firmware, and remaps keys.
//!
//! # Shape
//!
//! [`ControllerSupport`] names which screen the applet opens on and carries the
//! data that screen accepts, and [`ControllerSupport::show`] launches it. libnx
//! exposes one function per screen, all funnelling into a single private
//! launcher that then re-checks that the mode and the argument struct agree;
//! here the variant fixes both, so the pairs libnx rejects cannot be built.
//!
//! The launch itself is [`nx_service_applet::library_applet::launch`]; what this
//! crate owns is the two argument storages, the flags derived from the request,
//! and the meaning of the reply.
//!
//! # Versions
//!
//! This is the most version-dependent applet in the family: libnx consults the
//! running system version at eleven places. They fall into three groups, and
//! only one of them belongs to this crate.
//!
//! **Availability.** The strap guide and the firmware update need [3.0.0], key
//! remapping needs [11.0.0]. These are refusals, not protocol differences, and
//! they live in the caller: a service crate may not depend on the runtime that
//! holds the system version.
//!
//! **What the applet is told.** The system controller-support entry point skips
//! the HID service on pre-[3.0.0] and passes fixed values instead. That is a
//! difference in the [`ControllerSupportContext`] the caller supplies, so it too
//! stays outside.
//!
//! **What the applet speaks.** The remaining checks pick a library-applet API
//! version (3, 4, 5, 7 or 8) and one of two controller-support argument layouts
//! (four players before [8.0.0], eight from it on). Those two ladders are not
//! independent: [8.0.0] is a step on both, so every API version implies exactly
//! one layout. [`ControllerSupportVersion`] is that single ladder: one value per
//! step libnx actually distinguishes, naming both at once, and it is what
//! [`ControllerSupport::show`] takes. Grouping it that way is why there is one
//! `show` rather than one per screen per version: the five steps are a
//! parameter, not five APIs, and the layout can never be paired with the wrong
//! API version.
//!
//! # What it costs
//!
//! The applet runs as a separate process and blocks until the user leaves it, so
//! it must not be called from a context that cannot wait indefinitely, nor from
//! one where IPC may already be broken. The plain controller-support screen also
//! presents itself only when the request is not already satisfied, so a launch
//! that returns immediately is a normal outcome rather than a failure.
//!
//! # References
//!
//! - [Switchbrew Wiki: controller applet](https://switchbrew.org/wiki/Controller_Applet)
//! - [libnx hid_la.h](https://github.com/switchbrew/libnx/blob/master/nx/include/switch/applets/hid_la.h)

#![no_std]

extern crate nx_panic_handler as _; // provides #[panic_handler]

mod controller_support;
pub mod proto;

// The HID service type this crate's own API takes in. Re-exported so a consumer
// naming it does not have to depend on `nx-service-hid` for a type it only
// passes through.
pub use nx_service_hid::NpadJoyHoldType;

pub use self::controller_support::{
    ControllerSupport,
    ControllerSupportContext,
    ControllerSupportVersion,
    ShowError,
};
