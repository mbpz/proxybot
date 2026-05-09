#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_grpc_content_type() {
        // application/grpc
        let headers = vec![("content-type".to_string(), "application/grpc".to_string())];
        assert!(is_grpc_request(&headers));

        // application/grpc+proto
        let headers = vec![("content-type".to_string(), "application/grpc+proto".to_string())];
        assert!(is_grpc_request(&headers));

        // application/grpc-text
        let headers = vec![("content-type".to_string(), "application/grpc-text".to_string())];
        assert!(is_grpc_request(&headers));
    }

    #[test]
    fn test_detect_protobuf_content_type() {
        // application/x-protobuf
        let headers = vec![("content-type".to_string(), "application/x-protobuf".to_string())];
        assert!(is_protobuf(&headers));

        // application/vnd.google.protobuf
        let headers = vec![("content-type".to_string(), "application/vnd.google.protobuf".to_string())];
        assert!(is_protobuf(&headers));
    }

    #[test]
    fn test_case_insensitive() {
        let headers = vec![("Content-Type".to_string(), "APPLICATION/GRPC".to_string())];
        assert!(is_grpc_request(&headers));
    }
}