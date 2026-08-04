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

**Capture Session**:
The user-visible desktop lifecycle that observes, starts, and stops at most one MITM Runtime and reports actionable lifecycle failures consistently through window and tray Adapters.
_Avoid_: Proxy toggle, running flag

**Capture Event**:
A stable-id lifecycle event emitted by the MITM Runtime for a Captured Request, WebSocket frame, or failure.
_Avoid_: UI event, database row

**Application Attribution**:
The best-supported application identity assigned to a Captured Request, including confidence, source, and evidence.
_Avoid_: App tag, classification result

**DNS Observation**:
A client-scoped DNS query and its answers retained as possible evidence for Application Attribution.
_Avoid_: DNS log row, lookup cache

**Alert**:
A persisted security or anomaly fact published by detection logic and presented consistently through desktop and MCP adapters.
_Avoid_: Notification, finding row

**Alert Severity**:
The `Info`, `Warning`, or `Critical` urgency assigned when an Alert is published.
_Avoid_: Priority, level

**Alert Type**:
The stable category of an Alert: `NewDomain`, `NewIp`, `PrivacyExfil`, `AuthAnomaly`, or `UntrustedCert`.
_Avoid_: Title, event type

**Alert Acknowledgement**:
The persisted indication that an operator has reviewed an Alert; it does not delete or resolve the Alert.
_Avoid_: Dismissal, resolution

**Process Config**:
The immutable, validated ports, paths, and startup options assembled once before a ProxyBot process selects its desktop or MCP adapter.
_Avoid_: Global config, settings state

**Runtime Config**:
The MITM Runtime's bind, TLS, timeout, size, and reverse-target inputs derived from Process Config before the listener starts.
_Avoid_: App config, proxy state

**Inference Session**:
An immutable snapshot of one session's Inferred APIs and Captured Requests used as the shared input for generated outputs.
_Avoid_: Generator query, session rows

**Generated Artifact**:
A spec, mock, scaffold, vision-enhanced model, or deployment bundle derived from one Inference Session and written through validated naming and path rules.
_Avoid_: Generated file, generator result

**Runtime Extension Pipeline**:
The ordered request, response, connection, and traffic-effect Module that applies Plugin Dispatch Rules, plugin hooks, Rhai scripts, metrics, and Network Condition Rules for the MITM Runtime.
_Avoid_: Hook helper, desktop hook logic

**Captured Request Analysis**:
The immutable, normalized view of a Captured Request consumed by DAG, Graph, Topology, Auth, and anomaly analysis Implementations.
_Avoid_: Raw request, HTTP tuple, analysis row

**Desktop Network Resource**:
A desktop-owned listener or device whose lifecycle includes synchronous setup publication and completion-safe shutdown.
_Avoid_: Running flag, detached server, raw descriptor

## Relationships

- A **Rule File** contains zero or more **Routing Rules**
- A **Routing Rule** has exactly one **Rule Pattern**, one **Rule Action**, and one **Rule Priority**
- The first enabled **Routing Rule** whose **Rule Pattern** matches produces its **Rule Action**
- A **Plugin Dispatch Rule** maps an intercepted request to a plugin hook independently of **Routing Rules**
- A **Network Condition Rule** maps a host to a simulated network profile independently of **Routing Rules**
- The **MITM Runtime** turns client connections into zero or more **Captured Requests**
- One **Capture Session** observes and controls at most one **MITM Runtime**
- Window and tray Adapters publish the same **Capture Session** running state after successful lifecycle transitions
- A **Captured Request** is reported through one or more **Capture Events** with the same stable id
- Desktop persistence and UI delivery consume **Capture Events**; they are not part of the **MITM Runtime** protocol implementation
- A **Captured Request** may receive one **Application Attribution**
- A **DNS Observation** can support **Application Attribution** only for the same client and within the correlation window
- Anomaly detection and Auth state analysis publish **Alerts** with exactly one **Alert Severity** and one **Alert Type**
- SQLite assigns every **Alert** identifier and timestamp and owns its **Alert Acknowledgement**
- Desktop and MCP adapters query and acknowledge the same persisted **Alerts**
- One **Process Config** supplies every process-lifetime path and listener port to desktop or MCP adapters
- **Runtime Config** is derived from **Process Config**; it does not read environment variables or desktop state
- DNS upstream selection, TLS Rules, and spec-generation settings are mutable runtime state, not **Process Config**
- One **Inference Session** supplies the same Inferred APIs and Captured Requests to spec, mock, scaffold, vision, and deployment adapters
- A generation adapter transforms an **Inference Session** into a **Generated Artifact**; it does not reload or remap session persistence
- Default **Generated Artifact** paths stay under their configured roots, while explicit output paths remain operator-selected
- The **Runtime Extension Pipeline** matches enabled **Plugin Dispatch Rules** once, executes matching plugin hooks by ascending priority, then runs Rhai scripts by deterministic name order
- Successful plugin mutations accumulate; a panicking or timed-out plugin hook fails open, records an error metric, and does not commit its partial mutation
- Rhai rewrites accumulate in execution order, while a blocking script produces a stable 403 response
- The desktop Runtime Adapter translates MITM Runtime types and delegates extension behavior to the **Runtime Extension Pipeline**
- **Network Condition Rules** are evaluated by the **Runtime Extension Pipeline** when the MITM Runtime requests a traffic effect
- **Captured Request Analysis** converts accepted persisted timestamps to UTC once, maps invalid timestamps to Unix epoch, clamps invalid negative durations to zero, and preserves optional response status and device association
- Analysis time windows use epoch milliseconds and include both start and end boundaries
- DAG, Graph, Topology, Auth, and anomaly algorithms share **Captured Request Analysis** facts but retain independent Implementations and desktop wire contracts
- A retained **Desktop Network Resource** is published as running only after its listener binds or device setup completes successfully
- Repeated start of a running **Desktop Network Resource** fails, while stop is idempotent and returns only after owned tasks, listeners, and device handles are released
- The desktop composition root drains the **MITM Runtime**, DNS, dashboard, certificate distribution, and TUN **Desktop Network Resources** before process exit
- The **MITM Runtime** is the supported transport path; speculative transport and VPN Implementations without a composition-root path are not retained

## Example dialogue

> **Dev:** "If two **Routing Rules** match the same host, which **Rule Action** wins?"
> **Domain expert:** "The enabled rule with the lower **Rule Priority** is evaluated first, regardless of which **Rule File** contains it."

> **Dev:** "Can another device's **DNS Observation** identify this **Captured Request**?"
> **Domain expert:** "No — DNS evidence contributes to **Application Attribution** only when the client identity and timing match."

> **Dev:** "Can the desktop PF adapter and the **MITM Runtime** use different proxy ports?"
> **Domain expert:** "No — both receive the same **Process Config**, and the **MITM Runtime** derives its **Runtime Config** from it."

> **Dev:** "Can the mock and deployment generators observe different rows for the same run?"
> **Domain expert:** "No — each transformation receives one immutable **Inference Session** snapshot and does not query persistence itself."

## Flagged ambiguities

- "ruleset" refers to an external `RULE-SET` pattern target; persisted editable collections are **Rule Files**
- "rule" without a qualifier is ambiguous across routing, plugin dispatch, and network conditions
