---
name: "principle-open-closed"
description: "Open/Closed — add behaviour by adding a registry entry or a trait impl, not by editing logic that already works. Load when adding variants, extending behaviour across crates, or reviewing match chains"
type: "principle"
scope: "global"
---

# Open/Closed Principle (OCP)

## Rule

Software entities should be open for extension and resistant to modification of established behavior. Add new
behavior by adding an entry to a registry, a new type implementing an existing trait, or a new value handed to
a composition root — not by editing logic that already works.

Introduce an extension point when any of these signals fires:

1. **Cross-crate extension**: behavior is added by another crate. A provider crate contributes to the pipeline
   without the pipeline crate knowing it exists — it hands over a value implementing a trait the pipeline
   declares.
2. **Externally growing variant space**: the variants track something outside the repo (networks, providers,
   node implementations, wire formats). New variants must be additive.
3. **Repeated branching sites**: the same `match` or `if` over the same variant set appears in more than one
   place. Consolidate it behind one lookup.

When none of these fire — the variant set is fixed by a spec, matched in one place, and local to a crate — a
plain `match` is the clearer choice.

## Examples

1. **Registry entry over a branching chain**
   Which port name and which transport belong to a given Horizon service is an externally growing variant
   space — every firmware revision adds services. Model it as a table of specs plus one lookup, with
   per-variant behavior carried in the entry.

```rust
// ❌ Bad — every new service edits three proven functions, and the three can drift out of sync.
// The build that shipped had a service in `port_name_for` that `transport_for` had never heard
// of, so it fell through to CMIF, and every request to that TIPC-only port came back as an
// unhandled command id that the caller reported as a generic dispatch failure.
pub fn port_name_for(service: ServiceKind) -> Option<&'static str> {
    match service {
        ServiceKind::SetSys => Some("set:sys"),
        ServiceKind::FatalUser => Some("fatal:u"),
        _ => None,
    }
}

pub fn transport_for(service: ServiceKind) -> Transport {
    match service {
        ServiceKind::ProgramLauncher => Transport::Tipc,
        _ => Transport::Cmif, // a new TIPC-only service means editing this too
    }
}

pub fn describe_result(service: ServiceKind, rc: ResultCode) -> Option<&'static str> {
    match service {
        ServiceKind::SetSys => set_sys_result_name(rc),
        // ...and again here, in the one function every error report in the system goes through
    }
}
```

```rust
// ✅ Good — one spec per service, behavior included; adding a service is a new table entry,
// and no existing code is touched.
pub struct ServiceSpec {
    pub kind: ServiceKind,
    pub port_name: &'static str,
    pub transport: Transport,
    /// Names a result code from this service's own result namespace, if it defines one.
    pub describe_result: fn(ResultCode) -> Option<&'static str>,
}

pub const SERVICES: &[ServiceSpec] = &[
    ServiceSpec {
        kind: ServiceKind::SetSys,
        port_name: "set:sys",
        transport: Transport::Cmif,
        describe_result: set_sys_result_name,
    },
    // ...fatal:u, pgl, hid
];

pub fn spec_for(kind: ServiceKind) -> Option<&'static ServiceSpec> {
    SERVICES.iter().find(|spec| spec.kind == kind)
}
```

2. **Trait objects as the cross-crate seam**
   The IPC framework's client surface is open: the framework crate declares a trait, and every service
   client crate supplies an implementation. The framework never learns what the clients are.

```rust
// ❌ Bad — the framework imports every service client crate and hardcodes the set.
// The framework crate now depends on each of them, adding a client edits the framework,
// and the dependency graph gains a cycle the moment a client wants a framework type.
pub fn open_all(sm: &mut ServiceManager) -> Result<OpenedClients, OpenError> {
    let mut opened = OpenedClients::new();
    opened.push(nx_service_set::SetSysClient::open(sm)?);
    opened.push(nx_service_fatal::FatalClient::open(sm)?);
    if cfg!(feature = "error-context") {
        opened.push(nx_service_ectx::ErrorContextClient::open(sm)?);
    }
    Ok(opened)
}
```

```rust
// ✅ Good — the framework declares the seam; the runtime composes. Adding a client adds a
// crate and one line at the composition root, not an edit to the framework.
pub trait ServiceClient: Sync {
    fn port_name(&self) -> ServiceName;
    fn bind(&self, session: SessionHandle) -> Result<(), OpenError>;
}

pub struct BringUpBuilder {
    clients: ArrayVec<&'static dyn ServiceClient, MAX_CLIENTS>,
}

impl BringUpBuilder {
    pub fn with_client(mut self, client: &'static dyn ServiceClient) -> Self {
        self.clients.push(client);
        self
    }
}
```

## Why It Matters

Editing working code to add a variant is how regressions get introduced: the CMIF path is tested, the TIPC
path is tested, and the third `match` arm you added to both is the one that ships broken. A registry entry
cannot break the entries above it. A trait implementation cannot break the implementation next to it.

The three signals make this checkable instead of speculative. A service registry is open because the set of
Horizon services keeps growing with every firmware revision; a mapping onto HIPC's fixed set of buffer
descriptor kinds stays a closed `match`, because the ABI pins the variants and abstracting it would buy
nothing.

## Pragmatism Caveat

Not every `match` deserves a registry. If the variant set is frozen by an external spec (HIPC descriptor kinds,
the fixed set of SVC memory permissions), matched in one or two co-located places, and internal to a crate, a
`match` is clearer and cheaper than an indirection layer. An enum plus `match` also gives you exhaustiveness
checking that a registry table does not: the compiler will not tell you a table entry is missing.

When a signal fires and you still keep the branching, add a brief comment saying why (the ABI pins the
variants; the match sites are three lines apart). An undocumented violation is always wrong.

## Checklist

Before committing code, verify:

- [ ] New services, transports, allocators, or panic handlers are added as data entries or trait impls, not
      new branches
- [ ] Per-variant behavior lives in the variant's own entry (a function field or trait impl), not in a shared
      function's `match` arms
- [ ] Crates extend the IPC framework by handing it a value implementing its trait; the framework crate does
      not import the extending crates
- [ ] The same variant set is not matched on in more than one module, unless the sites are co-located and
      the variant set is frozen by an external spec
- [ ] A retained `match` over a spec-frozen variant set is exhaustive and documented as intentionally closed

## References

- [principle-single-responsibility](principle-single-responsibility.md) - Related: Extension points work only
  when each variant owns one concern
- [principle-dry-wet](principle-dry-wet.md) - Related: Extension points are the right home for genuinely shared
  behavior; flags on a shared helper are not
- [principle-least-surprise](principle-least-surprise.md) - Related: Registry entries must satisfy the same
  behavioral contract callers already expect

## External References

- [Understanding the Open/Closed Principle](https://dev.to/dazevedo/understanding-the-openclosed-principle-ocp-from-solid-keep-code-flexible-yet-stable-jo7)
