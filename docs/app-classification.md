# Application Attribution

ProxyBot can attach an Application Attribution to a Captured Request when the
available evidence supports one. Attribution is a best-effort debugging aid,
not proof that a process or package produced the traffic.

## Evidence

The attribution engine can use:

1. a canonicalized request host or TLS SNI matched against built-in domain and
   signature rules;
2. a user-defined custom TLS rule;
3. a recent DNS Observation from the same client, matched by host or resolved
   upstream IP.

DNS evidence is accepted only for the same client and within the configured
correlation window. An observation from another device must not identify the
request.

```text
DNS Observation(client, domain, answers, time)
                     |
TLS SNI / request host + upstream IP
                     |
                     v
Application Attribution(app, confidence, source, evidence)
```

## Built-in catalogue

The core includes domain rules for common social, commerce, media, developer,
payment, and AI services. The catalogue changes independently of product
versions; consult `proxybot-core/src/app_classifier.rs` for the current rules.

Domain matching is case-insensitive after IDNA normalization. A rule for
`example.com` matches the exact domain and its subdomains, but not
`example.com.attacker.test`.

Broad shared domains can produce false positives. The UI should always preserve
the attribution confidence, source, and evidence instead of presenting only an
app label.

## Custom rules

The desktop composition root loads custom TLS attribution rules from the Process
Config `app_rules_path`. The current file shape is the serialized
`CustomAppRule` model: an application id, display name, icon, confidence, and a
set of SNI, cipher-suite, or ALPN conditions.

This is an Advanced developer Interface and is not yet a stable public file
format. There is no mounted production UI for editing it. Back up the file before
changing it and expect schema changes before a stable release.

## Testing attribution

Use one of these paths:

- inspect the attribution fields on a Captured Request in Traffic;
- call the MCP `classify_request` tool with a host and optional SNI/DNS evidence;
- add a focused test to `proxybot-core` when changing built-in rules.

When contributing a domain rule, include:

- the application name;
- the exact domains observed;
- whether each domain appears to be an API, authentication, telemetry, CDN, or
  shared provider endpoint;
- how the evidence was collected without including captured credentials or
  private payloads;
- a positive test and a nearby false-positive test.

## Limitations

- Shared CDNs and identity providers often serve multiple applications.
- SNI and DNS reveal services, not necessarily the originating process.
- Encrypted ClientHello and some proxy/VPN paths reduce available evidence.
- Certificate pinning can prevent payload decryption even when attribution
  succeeds.
- DNS correlation requires the DNS Observation to be visible to ProxyBot.

Treat `Unknown` as a valid result when the evidence is insufficient.
