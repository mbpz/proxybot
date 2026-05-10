#[cfg(test)]
mod tests {
    use crate::protobuf::*;

    #[test]
    fn test_detect_grpc_content_type() {
        // application/grpc
        let headers = vec![("content-type".to_string(), "application/grpc".to_string())];
        assert!(is_grpc_request(&headers));

        // application/grpc+proto
        let headers = vec![(
            "content-type".to_string(),
            "application/grpc+proto".to_string(),
        )];
        assert!(is_grpc_request(&headers));

        // application/grpc-text
        let headers = vec![(
            "content-type".to_string(),
            "application/grpc-text".to_string(),
        )];
        assert!(is_grpc_request(&headers));
    }

    #[test]
    fn test_detect_protobuf_content_type() {
        // application/x-protobuf
        let headers = vec![(
            "content-type".to_string(),
            "application/x-protobuf".to_string(),
        )];
        assert!(is_protobuf(&headers));

        // application/vnd.google.protobuf
        let headers = vec![(
            "content-type".to_string(),
            "application/vnd.google.protobuf".to_string(),
        )];
        assert!(is_protobuf(&headers));
    }

    #[test]
    fn test_case_insensitive() {
        let headers = vec![("Content-Type".to_string(), "APPLICATION/GRPC".to_string())];
        assert!(is_grpc_request(&headers));
    }

    #[test]
    fn test_is_standard_grpc() {
        // Standard gRPC
        let headers = vec![("content-type".to_string(), "application/grpc".to_string())];
        assert!(is_standard_grpc(&headers));

        // gRPC-Web should NOT be detected as standard gRPC
        let headers = vec![(
            "content-type".to_string(),
            "application/grpc-web".to_string(),
        )];
        assert!(!is_standard_grpc(&headers));

        // gRPC-Web+proto should NOT be detected as standard gRPC
        let headers = vec![(
            "content-type".to_string(),
            "application/grpc-web+proto".to_string(),
        )];
        assert!(!is_standard_grpc(&headers));

        // application/grpc+proto IS standard gRPC
        let headers = vec![(
            "content-type".to_string(),
            "application/grpc+proto".to_string(),
        )];
        assert!(is_standard_grpc(&headers));
    }

    #[test]
    fn test_is_grpc_web() {
        let headers = vec![(
            "content-type".to_string(),
            "application/grpc-web".to_string(),
        )];
        assert!(is_grpc_web(&headers));

        let headers = vec![(
            "content-type".to_string(),
            "application/grpc-web+proto".to_string(),
        )];
        assert!(is_grpc_web(&headers));

        let headers = vec![("content-type".to_string(), "application/grpc".to_string())];
        assert!(!is_grpc_web(&headers));
    }

    #[test]
    fn test_is_any_grpc() {
        let headers = vec![("content-type".to_string(), "application/grpc".to_string())];
        assert!(is_any_grpc(&headers));

        let headers = vec![(
            "content-type".to_string(),
            "application/grpc-web".to_string(),
        )];
        assert!(is_any_grpc(&headers));
    }
}
