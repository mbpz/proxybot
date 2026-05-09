use std::future::Future;
use std::pin::Pin;

use crate::error::AppError;
use crate::proxy::InterceptedRequest;

/// Boxed future type alias for async hooks
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Hook points for plugin callbacks
pub struct PluginHooks {
    pub on_request: Option<Box<dyn Fn(&mut InterceptedRequest) + Send + Sync>>,
    pub on_response: Option<Box<dyn Fn(&mut InterceptedResponse) + Send + Sync>>,
    pub on_connect: Option<Box<dyn Fn(&str) -> ConnectDecision + Send + Sync>>,
    pub on_error: Option<Box<dyn Fn(&AppError) + Send + Sync>>,
    // Async variants
    pub on_request_async: Option<Box<dyn Fn(&mut InterceptedRequest) -> BoxFuture<'static, ()> + Send + Sync>>,
    pub on_response_async: Option<Box<dyn Fn(&mut InterceptedResponse) -> BoxFuture<'static, ()> + Send + Sync>>,
}

impl Default for PluginHooks {
    fn default() -> Self {
        Self {
            on_request: None,
            on_response: None,
            on_connect: None,
            on_error: None,
            on_request_async: None,
            on_response_async: None,
        }
    }
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
    fn config_schema(&self) -> Option<&'static str> { None }
}

// InterceptedResponse for on_response plugin hooks
#[derive(Debug, Clone, Default)]
pub struct InterceptedResponse {
    pub status: Option<u16>,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}