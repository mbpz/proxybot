//! Application Attribution Module.
//!
//! One Interface owns application catalog precedence, hostname/SNI
//! classification, DNS observation retention, same-client correlation, and
//! attribution evidence. Desktop, MCP, and transport callers are Adapters.

use crate::{
    canonicalize_host, AppMatch, AppRule, ApplicationClassifier, CustomAppRule, DnsObservation,
    MatchSource,
};
use std::collections::VecDeque;
use std::sync::{Mutex, RwLock};

pub const DEFAULT_DNS_CORRELATION_WINDOW_MS: u64 = 300_000;
pub const DEFAULT_DNS_OBSERVATION_CAPACITY: usize = 10_000;

/// Evidence available when attributing one Captured Request or connection.
#[derive(Clone, Copy, Debug)]
pub struct AttributionInput<'a> {
    pub host: &'a str,
    pub sni: Option<&'a str>,
    pub client_ip: Option<&'a str>,
    pub upstream_ip: Option<&'a str>,
    pub captured_at_ms: u64,
}

/// Stateful application attribution with one canonical classifier and DNS log.
pub struct AttributionEngine {
    classifier: RwLock<ApplicationClassifier>,
    observations: Mutex<VecDeque<DnsObservation>>,
    capacity: usize,
    correlation_window_ms: u64,
}

impl AttributionEngine {
    pub fn new(
        classifier: ApplicationClassifier,
        capacity: usize,
        correlation_window_ms: u64,
    ) -> Self {
        Self {
            classifier: RwLock::new(classifier),
            observations: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
            correlation_window_ms,
        }
    }

    /// Record a DNS Observation. Oldest observations are evicted at capacity.
    pub fn observe_dns(&self, observation: DnsObservation) {
        let mut observations = self.observations.lock().expect("DNS observations poisoned");
        if observations.len() >= self.capacity {
            observations.pop_front();
        }
        observations.push_back(observation);
    }

    /// Attribute one input using custom/SNI/domain evidence before same-client DNS.
    pub fn classify(&self, input: AttributionInput<'_>) -> Option<AppMatch> {
        if let Some(attribution) = self
            .classifier
            .read()
            .expect("application classifier poisoned")
            .classify_request(input.host, input.sni, None)
        {
            return Some(attribution);
        }

        self.correlate_dns(input)
    }

    /// Attribute from retained DNS evidence only.
    pub fn correlate_dns(&self, input: AttributionInput<'_>) -> Option<AppMatch> {
        let client_ip = input.client_ip;
        let host = canonicalize_host(input.host);
        let observations = self.observations.lock().ok()?;
        for observation in observations.iter().rev() {
            if input.captured_at_ms < observation.timestamp_ms {
                continue;
            }
            if input.captured_at_ms - observation.timestamp_ms > self.correlation_window_ms {
                break;
            }
            if observation.client_ip.as_deref() != client_ip {
                continue;
            }

            let ip_match = input.upstream_ip.is_some_and(|upstream_ip| {
                observation
                    .resolved_ips
                    .iter()
                    .any(|resolved_ip| resolved_ip == upstream_ip)
            });
            let host_match = host.as_ref().is_some_and(|host| {
                canonicalize_host(&observation.domain)
                    .is_some_and(|domain| host == &domain || host.ends_with(&format!(".{domain}")))
            });
            if !ip_match && !host_match {
                continue;
            }

            if let Some(mut attribution) = self
                .classifier
                .read()
                .expect("application classifier poisoned")
                .classify_domain(&observation.domain)
            {
                attribution.source = MatchSource::Dns;
                attribution.confidence = 0.7;
                attribution.evidence = vec![if ip_match {
                    format!(
                        "dns:{}->{}",
                        observation.domain,
                        input.upstream_ip.unwrap_or_default()
                    )
                } else {
                    format!("dns:{}", observation.domain)
                }];
                return Some(attribution);
            }

            if let Some(app_name) = observation.app_name.as_ref() {
                return Some(AppMatch {
                    app_id: application_id(app_name),
                    app_name: app_name.clone(),
                    app_icon: observation.app_icon.clone(),
                    confidence: 0.7,
                    source: MatchSource::Dns,
                    evidence: vec![format!("dns:{}", observation.domain)],
                });
            }
        }
        None
    }

    /// Atomically replace custom rules used by subsequent classifications.
    pub fn replace_custom_rules(&self, custom_rules: Vec<CustomAppRule>) {
        let mut classifier = self
            .classifier
            .write()
            .expect("application classifier poisoned");
        let domain_rules: Vec<AppRule> = classifier.domain_rules().to_vec();
        *classifier = ApplicationClassifier::with_rules(domain_rules, custom_rules);
    }

    pub fn classify_domain(&self, domain: &str) -> Option<AppMatch> {
        self.classifier
            .read()
            .expect("application classifier poisoned")
            .classify_domain(domain)
    }

    pub fn observations(&self, limit: usize) -> Vec<DnsObservation> {
        self.observations
            .lock()
            .expect("DNS observations poisoned")
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn clear_observations(&self) {
        self.observations
            .lock()
            .expect("DNS observations poisoned")
            .clear();
    }
}

impl Default for AttributionEngine {
    fn default() -> Self {
        Self::new(
            ApplicationClassifier::default(),
            DEFAULT_DNS_OBSERVATION_CAPACITY,
            DEFAULT_DNS_CORRELATION_WINDOW_MS,
        )
    }
}

fn application_id(name: &str) -> String {
    let mut id = String::new();
    let mut separator = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if separator && !id.is_empty() {
                id.push('-');
            }
            id.push(character.to_ascii_lowercase());
            separator = false;
        } else {
            separator = true;
        }
    }
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(client_ip: &str, timestamp_ms: u64) -> DnsObservation {
        DnsObservation {
            domain: "api.weixin.qq.com".to_owned(),
            timestamp_ms,
            app_name: Some("WeChat".to_owned()),
            app_icon: Some("💬".to_owned()),
            action: None,
            resolved_ips: vec!["1.2.3.4".to_owned(), "2001:db8::1".to_owned()],
            client_ip: Some(client_ip.to_owned()),
        }
    }

    #[test]
    fn dns_ip_correlation_is_same_client_and_supports_ipv6() {
        let engine = AttributionEngine::default();
        engine.observe_dns(observation("192.168.1.2", 1_000));
        let input = |client_ip, upstream_ip| AttributionInput {
            host: upstream_ip,
            sni: None,
            client_ip: Some(client_ip),
            upstream_ip: Some(upstream_ip),
            captured_at_ms: 1_100,
        };

        assert!(engine.classify(input("192.168.1.3", "1.2.3.4")).is_none());
        let ipv6 = engine
            .classify(input("192.168.1.2", "2001:db8::1"))
            .unwrap();
        assert_eq!(ipv6.app_name, "WeChat");
        assert_eq!(ipv6.source, MatchSource::Dns);
    }

    #[test]
    fn dns_correlation_respects_window_and_future_observations() {
        let engine = AttributionEngine::new(ApplicationClassifier::default(), 10, 100);
        engine.observe_dns(observation("client", 1_000));
        let input = |captured_at_ms| AttributionInput {
            host: "1.2.3.4",
            sni: None,
            client_ip: Some("client"),
            upstream_ip: Some("1.2.3.4"),
            captured_at_ms,
        };
        assert!(engine.classify(input(999)).is_none());
        assert!(engine.classify(input(1_101)).is_none());
        assert!(engine.classify(input(1_100)).is_some());
    }

    #[test]
    fn direct_domain_evidence_precedes_dns() {
        let engine = AttributionEngine::default();
        engine.observe_dns(observation("client", 1_000));
        let attribution = engine
            .classify(AttributionInput {
                host: "API.OPENAI.COM.",
                sni: None,
                client_ip: Some("client"),
                upstream_ip: Some("1.2.3.4"),
                captured_at_ms: 1_100,
            })
            .unwrap();
        assert_eq!(attribution.app_name, "OpenAI");
        assert_eq!(attribution.source, MatchSource::Domain);
    }

    #[test]
    fn replacing_custom_rules_changes_the_next_attribution() {
        let engine = AttributionEngine::default();
        let input = AttributionInput {
            host: "api.tiktokv.com",
            sni: Some("api.tiktokv.com"),
            client_ip: None,
            upstream_ip: None,
            captured_at_ms: 1,
        };
        assert_eq!(engine.classify(input).unwrap().app_id, "douyin");

        engine.replace_custom_rules(vec![CustomAppRule {
            app_id: "internal-video".to_owned(),
            app_name: "Internal Video".to_owned(),
            icon: "I".to_owned(),
            conditions: vec![crate::RuleCondition::Sni {
                pattern: "*.tiktokv.com".to_owned(),
            }],
            confidence: 0.8,
        }]);
        let attribution = engine.classify(input).unwrap();
        assert_eq!(attribution.app_id, "internal-video");
        assert_eq!(attribution.source, MatchSource::Custom);
    }
}
