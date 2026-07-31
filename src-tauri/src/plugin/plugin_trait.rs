use std::future::Future;
use std::pin::Pin;

use crate::error::AppError;
use crate::proxy::InterceptedRequest;

/// Boxed future type alias for async hooks
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type RequestHook = Box<dyn Fn(&mut InterceptedRequest) + Send + Sync>;
pub type ResponseHook = Box<dyn Fn(&mut InterceptedResponse) + Send + Sync>;
pub type ConnectHook = Box<dyn Fn(&str) -> ConnectDecision + Send + Sync>;
pub type ErrorHook = Box<dyn Fn(&AppError) + Send + Sync>;
pub type AsyncRequestHook =
    Box<dyn Fn(&mut InterceptedRequest) -> BoxFuture<'static, ()> + Send + Sync>;
pub type AsyncResponseHook =
    Box<dyn Fn(&mut InterceptedResponse) -> BoxFuture<'static, ()> + Send + Sync>;

/// Hook points for plugin callbacks
#[derive(Default)]
pub struct PluginHooks {
    pub on_request: Option<RequestHook>,
    pub on_response: Option<ResponseHook>,
    pub on_connect: Option<ConnectHook>,
    pub on_error: Option<ErrorHook>,
    // Async variants
    pub on_request_async: Option<AsyncRequestHook>,
    pub on_response_async: Option<AsyncResponseHook>,
}
#[derive(Debug)]
pub enum ConnectDecision {
    Allow,
    Block,
    Redirect(String),
}

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn hooks(&self) -> PluginHooks;
    fn config_schema(&self) -> Option<&'static str> {
        None
    }
}

// InterceptedResponse for on_response plugin hooks
#[derive(Debug, Clone, Default)]
pub struct InterceptedResponse {
    pub status: Option<u16>,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}
