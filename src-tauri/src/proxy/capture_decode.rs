//! Payload enrichment applied by the desktop Capture Event Adapter.

use crate::graphql::GraphQLDecoder;
use crate::protobuf;

/// Decode a captured gRPC response into the JSON representation shown by the UI.
pub(super) fn try_decode_grpc_body(
    response_headers: &[(String, String)],
    response_body: &[u8],
) -> Option<String> {
    if !protobuf::is_grpc_request(response_headers) || response_body.is_empty() {
        return None;
    }

    if !protobuf::is_standard_grpc(response_headers) {
        return protobuf::decode_protobuf(response_body).ok();
    }

    let decoded_frames = protobuf::decode_grpc_frames(response_body)
        .iter()
        .enumerate()
        .filter_map(|(index, frame)| {
            protobuf::decode_protobuf(&frame.data)
                .ok()
                .filter(|decoded| decoded != "[]")
                .map(|decoded| format!(r#"{{"frame":{index},"decoded":{decoded}}}"#))
        })
        .collect::<Vec<_>>();

    if decoded_frames.is_empty() {
        protobuf::decode_protobuf(response_body).ok()
    } else {
        Some(format!("[{}]", decoded_frames.join(",")))
    }
}

/// Parse a captured GraphQL request into the structured representation shown by the UI.
pub(super) fn try_decode_graphql_body(
    request_headers: &[(String, String)],
    request_body: Option<&str>,
) -> Option<String> {
    let body = request_body.filter(|body| !body.is_empty())?;
    if !GraphQLDecoder::is_graphql_content_type(request_headers)
        || !GraphQLDecoder::is_graphql_body(body)
    {
        return None;
    }
    let operation = GraphQLDecoder::parse_request(body).ok()?;
    serde_json::to_string(&operation).ok()
}
