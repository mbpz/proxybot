# ProxyBot

ProxyBot captures developer traffic and applies routing rules before forwarding or intercepting it.

## Language

**Captured Request**:
An HTTP or WebSocket exchange observed by ProxyBot and attributed to a device or application when possible.
_Avoid_: Traffic item, request row

**Rule File**:
A named YAML document containing one file-scoped collection of routing rules.
_Avoid_: Ruleset, preset

**Routing Rule**:
A configurable match-and-action definition with an enabled state and evaluation priority.
_Avoid_: Filter, policy

**Rule Pattern**:
The domain, IP, geography, or referenced-set condition of a routing rule.
_Avoid_: Matcher, selector

**Rule Action**:
The routing, mapping, rejection, or breakpoint outcome produced by a matching routing rule.
_Avoid_: Operation, behavior

**Rule Priority**:
The numeric precedence of a routing rule, where lower values are evaluated first.
_Avoid_: Position, rank

**Plugin Dispatch Rule**:
A request-pattern mapping that selects a plugin hook; it does not produce a routing action.
_Avoid_: Routing Rule, plugin routing rule

**Network Condition Rule**:
A host-pattern mapping that selects a simulated network profile; it does not route traffic.
_Avoid_: Routing Rule, network rule

**MITM Runtime**:
The running proxy that accepts client connections, terminates or bypasses TLS, applies Routing Rules and hooks, forwards transactions, and emits capture events.
_Avoid_: Proxy state, listener wrapper

**Capture Event**:
A stable-id lifecycle event emitted by the MITM Runtime for a Captured Request, WebSocket frame, or failure.
_Avoid_: UI event, database row

**Application Attribution**:
The best-supported application identity assigned to a Captured Request, including confidence, source, and evidence.
_Avoid_: App tag, classification result

**DNS Observation**:
A client-scoped DNS query and its answers retained as possible evidence for Application Attribution.
_Avoid_: DNS log row, lookup cache

**Process Config**:
The immutable, validated ports, paths, and startup options assembled once before a ProxyBot process selects its desktop or MCP adapter.
_Avoid_: Global config, settings state

**Runtime Config**:
The MITM Runtime's bind, TLS, timeout, size, and reverse-target inputs derived from Process Config before the listener starts.
_Avoid_: App config, proxy state

## Relationships

- A **Rule File** contains zero or more **Routing Rules**
- A **Routing Rule** has exactly one **Rule Pattern**, one **Rule Action**, and one **Rule Priority**
- The first enabled **Routing Rule** whose **Rule Pattern** matches produces its **Rule Action**
- A **Plugin Dispatch Rule** maps an intercepted request to a plugin hook independently of **Routing Rules**
- A **Network Condition Rule** maps a host to a simulated network profile independently of **Routing Rules**
- The **MITM Runtime** turns client connections into zero or more **Captured Requests**
- A **Captured Request** is reported through one or more **Capture Events** with the same stable id
- Desktop persistence and UI delivery consume **Capture Events**; they are not part of the **MITM Runtime** protocol implementation
- A **Captured Request** may receive one **Application Attribution**
- A **DNS Observation** can support **Application Attribution** only for the same client and within the correlation window
- One **Process Config** supplies every process-lifetime path and listener port to desktop or MCP adapters
- **Runtime Config** is derived from **Process Config**; it does not read environment variables or desktop state
- DNS upstream selection, TLS Rules, and spec-generation settings are mutable runtime state, not **Process Config**

## Example dialogue

> **Dev:** "If two **Routing Rules** match the same host, which **Rule Action** wins?"
> **Domain expert:** "The enabled rule with the lower **Rule Priority** is evaluated first, regardless of which **Rule File** contains it."

> **Dev:** "Can another device's **DNS Observation** identify this **Captured Request**?"
> **Domain expert:** "No — DNS evidence contributes to **Application Attribution** only when the client identity and timing match."

> **Dev:** "Can the desktop PF adapter and the **MITM Runtime** use different proxy ports?"
> **Domain expert:** "No — both receive the same **Process Config**, and the **MITM Runtime** derives its **Runtime Config** from it."

## Flagged ambiguities

- "ruleset" refers to an external `RULE-SET` pattern target; persisted editable collections are **Rule Files**
- "rule" without a qualifier is ambiguous across routing, plugin dispatch, and network conditions
