# Applet Type Resolution

This document describes how a Switch homebrew binary declares what *kind* of
applet it is, how that decision drives the AM (Applet Manager) handshake, and
how nx-std mirrors libnx's mechanism while remaining `no_std`.

## Table of Contents

- [Overview](#overview)
- [The Applet Type Enumeration](#the-applet-type-enumeration)
- [How libnx Decides: Weak Symbol + Homebrew ABI](#how-libnx-decides-weak-symbol--homebrew-abi)
  - [Path A — NRO Loaded via hbloader](#path-a--nro-loaded-via-hbloader)
  - [Path B — NSO Sysmodule](#path-b--nso-sysmodule)
- [How the Type Drives `appletOE` vs `appletAE`](#how-the-type-drives-appletoe-vs-appletae)
- [How nx-std Mirrors This](#how-nx-std-mirrors-this)
- [Building an Application vs an Applet](#building-an-application-vs-an-applet)
- [References](#references)

## Overview

Every process running under Horizon OS that talks to AM must tell AM *which
slot* it occupies — regular application, system applet, library applet,
overlay applet, system application, or none at all. This selection happens
**once, at process startup**, before AM is contacted, and is immutable for the
lifetime of the process.

The selection drives three observable behaviors:

1. **Service name**: `appletOE` (`IApplicationProxyService`) for
   `Application`; `appletAE` (`IAllSystemAppletProxiesService`) for everything
   else.
2. **Proxy command ID**: `0` / `100` / `200`·`201` / `300` / `350` on the
   chosen service.
3. **Per-type sub-interface availability**: `IApplicationFunctions` only for
   `Application`, `IApplicationCreator` only for `SystemApplet`,
   `ILibraryAppletSelfAccessor` only for `LibraryApplet` (pre-15.0.0), etc.

## The Applet Type Enumeration

The values are stable across libnx and nx-std and originate from Nintendo's
`AppletType` enum:

| Value | Variant             | Service     | Proxy cmd            | Typical binary               |
|------:|---------------------|-------------|----------------------|------------------------------|
|   −1  | `Default`           | (coerced to `Application`) | —    | weak default in libnx        |
|    0  | `Application`       | `appletOE`  | `0`                  | games, regular homebrew NRO  |
|    1  | `SystemApplet`      | `appletAE`  | `100`                | qlaunch, HOME menu           |
|    2  | `LibraryApplet`     | `appletAE`  | `200` / `201` (3.0+) | swkbd, error dialog          |
|    3  | `OverlayApplet`     | `appletAE`  | `300`                | broadcasting overlay         |
|    4  | `SystemApplication` | `appletAE`  | `350`                | installed system titles      |
|  *n/a*| `None`              | (skip AM)   | —                    | background sysmodules        |

`Default` is the weak-symbol default in libnx; `_appletInitialize` coerces it
to `Application` before doing anything. nx-service-applet does the same in
`connect()`.

`None` is special — it short-circuits the entire AM handshake. Background
sysmodules (those without UI affinity) declare this so they don't open any
proxy and don't burn an AM session.

## How libnx Decides: Weak Symbol + Homebrew ABI

The mechanism is one global variable, declared `weak` so binaries can override
it at link time (`subprojects/libnx/src/nx/source/services/applet.c:9`):

```c
__attribute__((weak)) u32 __nx_applet_type = AppletType_Default;
```

There is **no runtime detection** of "am I a game or a library applet."
The binary declares its identity through one of two channels.

### Path A — NRO Loaded via hbloader

When an NRO launches via the homebrew ABI (e.g. through hbmenu or the album
applet), `crt0` calls `envSetup(ctx, …)` where `ctx` points to a list of
`ConfigEntry` records the loader emitted. The loader tells the NRO what slot
it was launched into
(`subprojects/libnx/src/nx/source/runtime/env.c:81-84`):

```c
case EntryType_AppletType:
    __nx_applet_type = ent->Value[0];
    if ((ent->Value[1] & EnvAppletFlags_ApplicationOverride) &&
        __nx_applet_type == AppletType_SystemApplication)
        __nx_applet_type = AppletType_Application;
    break;
```

The `ApplicationOverride` flag (bit 0 of `flags`) handles the title-takeover
case: hbloader may run inside a `SystemApplication` slot but want the
homebrew to *behave* as a regular `Application`. The flag downgrades the
declared type so the rest of libnx sees `Application` everywhere.

This path is how a *single NRO build* can run as either a library applet
(launched from the album → `LibraryApplet`) or a full application (after the
title-takeover hack → `Application`): the type is supplied by whoever loads
the NRO, not baked into the NRO itself.

### Path B — NSO Sysmodule

When the process is an NSO (built-in sysmodule, replacement title installed
to the system), `envSetup` is called with `ctx == NULL` — there is no
homebrew ABI block to parse, so `__nx_applet_type` keeps its weak default of
`Default`.

To run as anything other than an application, an NSO binary defines its own
non-weak symbol that **overrides** the libnx default at link time:

```c
u32 __nx_applet_type = AppletType_SystemApplet;       // qlaunch-style
u32 __nx_applet_type = AppletType_OverlayApplet;      // overlay
u32 __nx_applet_type = AppletType_LibraryApplet;      // library applet
u32 __nx_applet_type = AppletType_SystemApplication;  // installed title
u32 __nx_applet_type = AppletType_None;               // pure background sysmodule
```

The same weak-override technique is used for sister knobs that customize the
AM init:

| Symbol                              | Purpose                                                     |
|-------------------------------------|-------------------------------------------------------------|
| `__nx_applet_auto_notifyrunning`    | Skip `appletNotifyRunning` if false                         |
| `__nx_applet_AppletAttribute`       | Payload for `OpenLibraryAppletProxy` (HOS 3.0.0+)           |
| `__nx_applet_PerformanceConfiguration[2]` | Per-mode config for `apmSetPerformanceConfiguration` |
| `__nx_applet_exit_mode`             | Whether `appletExit` runs exit cmds (NSO vs NRO)            |
| `__nx_applet_init_timeout`          | Timeout for the LibraryApplet `AM_BUSY_ERROR` retry loop    |

All weak, all decided at link time, all consumed by `_appletInitialize`.

## How the Type Drives `appletOE` vs `appletAE`

By the time `appletInitialize()` runs (called from libnx's `__libnx_initheap`
→ `__appInit` chain at
`subprojects/libnx/src/nx/source/runtime/init.c:124`), `__nx_applet_type` is
already final. The dispatch itself is trivial
(`subprojects/libnx/src/nx/source/services/applet.c:120-148`):

```c
if (__nx_applet_type == AppletType_None)
    return 0;                                          // skip AM entirely

switch (__nx_applet_type) {
    case AppletType_Default:                           // weak default
        __nx_applet_type = AppletType_Application;     // coerce + fall through
    case AppletType_Application:
        rc = smGetService(&g_appletSrv, "appletOE");
        break;
    default:                                           // SystemApplet,
        rc = smGetService(&g_appletSrv, "appletAE");   //   LibraryApplet,
        break;                                         //   OverlayApplet,
}                                                      //   SystemApplication
```

`None` is the only path that returns *before* opening any service. Every
other path opens exactly one of `appletOE` or `appletAE`, converts it to a
domain, and proceeds with the proxy handshake described in the
[`nx-service-applet` crate docs](../subprojects/nx-service-applet/src/lib.rs).

## How nx-std Mirrors This

nx-std splits libnx's monolithic "init reads a global" into two explicit
layers:

### Layer 1 — `nx-rt` parses the homebrew ABI

`nx-rt`'s env module parses the same `EntryType_AppletType` records that
libnx parses, with the same `ApplicationOverride` flag handling
(`subprojects/nx-rt/src/env/config.rs:218-259`):

```rust
#[repr(i32)]
pub enum AppletType {
    Default = -1,
    Application = 0,
    SystemApplet = 1,
    LibraryApplet = 2,
    OverlayApplet = 3,
    SystemApplication = 4,
}

impl AppletType {
    const FLAG_APPLICATION_OVERRIDE: u64 = 1 << 0;

    pub const fn from_raw(value: u32, flags: u64) -> Self {
        let mut applet_type = match value { /* … */ };
        if (flags & Self::FLAG_APPLICATION_OVERRIDE) != 0
            && matches!(applet_type, Self::SystemApplication)
        {
            applet_type = Self::Application;
        }
        applet_type
    }
}
```

The parsed value lives in `nx-rt`'s env state and is reachable via
`nx_rt::env::applet_type()`.

### Layer 2 — `nx-service-applet` takes the type as a parameter

The crate has no global. `connect()` accepts an `AppletType` argument and
performs the same coercion + service selection as libnx
(`subprojects/nx-service-applet/src/lib.rs:930-955`):

```rust
pub fn connect(
    sm: &SmService,
    applet_type: AppletType,
) -> Result<Option<AppletService>, ConnectError> {
    if matches!(applet_type, AppletType::None) {
        return Ok(None);                           // skip AM
    }

    let applet_type = if matches!(applet_type, AppletType::Default) {
        AppletType::Application                    // mirror libnx coercion
    } else {
        applet_type
    };

    let service_name = if applet_type.uses_applet_oe() {
        SERVICE_NAME_OE                            // "appletOE"
    } else {
        SERVICE_NAME_AE                            // "appletAE"
    };
    /* … */
}
```

### Why this layout

Pushing the "what am I?" decision up to the caller, instead of reading a
weak global, has two practical benefits in a `no_std` Rust workspace:

1. **Globals tied to weak linkage don't compose well across crates**. A weak
   symbol in one rlib can be silently shadowed by another rlib that depends
   on it, leading to action-at-a-distance bugs that only surface at link
   time. Passing the value explicitly makes the dependency a first-class
   function argument.
2. **`nx-service-applet` becomes testable in isolation**. Without a hidden
   global, each test can pick the applet type it wants without resorting to
   link-time tricks.

`nx-rt` plays the role of "the libnx runtime": it sources the type from the
ABI, holds the state, and is the only crate that talks to
`nx_service_applet::connect`. Consumers of `nx-rt` call its public
applet-init entry point and never see `AppletType` directly unless they want
to.

## Building an Application vs an Applet

The matrix from the user's perspective:

| You want to build…                      | How                                                                                   |
|------------------------------------------|---------------------------------------------------------------------------------------|
| **Regular game / homebrew NRO**          | Default. Loader supplies `AppletType_Application` (or `LibraryApplet` for album-launch). |
| **Library applet** (swkbd-style)         | Define `u32 __nx_applet_type = AppletType_LibraryApplet;` in your binary, supply `__nx_applet_AppletAttribute` for HOS 3.0.0+ behavior. |
| **System applet** (qlaunch replacement)  | Define `u32 __nx_applet_type = AppletType_SystemApplet;`. Requires NSO + title install. |
| **Overlay applet**                       | Define `u32 __nx_applet_type = AppletType_OverlayApplet;`. NSO + title install.       |
| **System application** (installed title) | Define `u32 __nx_applet_type = AppletType_SystemApplication;`.                        |
| **Pure background sysmodule** (no AM)    | Define `u32 __nx_applet_type = AppletType_None;`. AM is skipped entirely.             |

For nx-std consumers, the equivalent is calling `nx_rt::env::applet_type()`
(or whatever entry point your runtime exposes) and passing the value into
the applet init path. The build system itself does not need per-mode
configuration — the same crate set works for every type.

## References

- libnx `applet.c` — `_appletInitialize` and weak globals
  (`subprojects/libnx/src/nx/source/services/applet.c`)
- libnx `env.c` — homebrew ABI parser
  (`subprojects/libnx/src/nx/source/runtime/env.c`)
- libnx `init.c` — call order during process startup
  (`subprojects/libnx/src/nx/source/runtime/init.c`)
- `nx-service-applet` crate root docs — proxy/sub-interface layout
  (`subprojects/nx-service-applet/src/lib.rs`)
- `nx-rt` env config — Rust-side ABI parser
  (`subprojects/nx-rt/src/env/config.rs`)
- `nx-rt` applet service — Rust-side init equivalent of `_appletInitialize`
  (`subprojects/nx-rt/src/services/applet.rs`)
- [Switchbrew Wiki: Applet Manager services](https://switchbrew.org/wiki/Applet_Manager_services)
