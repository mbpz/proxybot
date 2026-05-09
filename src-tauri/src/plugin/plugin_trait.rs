use crate::error::AppError;
use crate::proxy::InterceptedRequest;

/// Hook points for plugin callbacks
#[derive(Default)]
pub struct PluginHooks {
    pub on_request: Option<Box<dyn Fn(&mut InterceptedRequest) + Send + Sync>>,
    pub on_response: Option<Box<dyn Fn(&mut InterceptedResponse) + Send + Sync>>,
    pub on_connect: Option<Box<dyn Fn(&str) -> ConnectDecision + Send + Sync>>,
    pub on_error: Option<Box<dyn Fn(&AppError) + Send + Sync>>,
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

// Stub for InterceptedResponse (placeholder until proxy.rs defines it)
#[derive(Debug, Clone, Default)]
pub struct InterceptedResponse {
    pub status: Option<u16>,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}