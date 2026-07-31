pub mod server;
pub use server::DashboardServer;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::InterceptedRequest;
    use std::collections::HashSet;

    #[test]
    fn test_dashboard_new() {
        let dashboard = DashboardServer::new(0);
        assert!(!dashboard.is_running());
        assert_eq!(dashboard.port(), 0);
        assert!(!dashboard.token().is_empty());
    }

    #[test]
    fn test_push_request() {
        let dashboard = DashboardServer::new(0);
        let req = InterceptedRequest {
            id: "test-1".into(),
            timestamp: "1234567890".into(),
            method: "GET".into(),
            host: "example.com".into(),
            path: "/".into(),
            scheme: "https".into(),
            ..Default::default()
        };
        dashboard.push_request(req);
        let requests = dashboard.requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].host, "example.com");
    }

    #[test]
    fn test_ring_buffer_eviction() {
        let dashboard = DashboardServer::new(0);
        for i in 0..1005 {
            dashboard.push_request(InterceptedRequest {
                id: format!("req-{}", i),
                timestamp: "1234567890".into(),
                method: "GET".into(),
                host: "example.com".into(),
                path: "/".into(),
                scheme: "https".into(),
                ..Default::default()
            });
        }
        let requests = dashboard.requests.lock().unwrap();
        assert_eq!(requests.len(), 1000);
        // Oldest should be evicted
        assert_eq!(requests[0].id, "req-5");
    }

    #[test]
    fn test_token_uniqueness() {
        let tokens: HashSet<_> = (0..128)
            .map(|_| DashboardServer::new(0).token().to_owned())
            .collect();

        assert_eq!(tokens.len(), 128);
        assert!(tokens.iter().all(|token| token.len() == 32));
    }
}
