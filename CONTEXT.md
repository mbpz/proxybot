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

## Relationships

- A **Rule File** contains zero or more **Routing Rules**
- A **Routing Rule** has exactly one **Rule Pattern**, one **Rule Action**, and one **Rule Priority**
- The first enabled **Routing Rule** whose **Rule Pattern** matches produces its **Rule Action**
- A **Plugin Dispatch Rule** maps an intercepted request to a plugin hook independently of **Routing Rules**
- A **Network Condition Rule** maps a host to a simulated network profile independently of **Routing Rules**

## Example dialogue

> **Dev:** "If two **Routing Rules** match the same host, which **Rule Action** wins?"
> **Domain expert:** "The enabled rule with the lower **Rule Priority** is evaluated first, regardless of which **Rule File** contains it."

## Flagged ambiguities

- "ruleset" refers to an external `RULE-SET` pattern target; persisted editable collections are **Rule Files**
- "rule" without a qualifier is ambiguous across routing, plugin dispatch, and network conditions
