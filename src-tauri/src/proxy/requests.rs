//! Desktop enrichment Adapter for Captured Request device attribution.

use super::DeviceContext;
use crate::db::DbState;
use std::sync::Arc;

/// Get or register the desktop device associated with a client IP.
pub(super) async fn get_or_create_device(
    db_state: &Arc<DbState>,
    ip_address: &str,
) -> Option<DeviceContext> {
    if let Some(device) = db_state.get_device_by_ip_internal(ip_address) {
        return Some(DeviceContext {
            device_id: device.id,
            device_name: device.name,
            ip_address: ip_address.to_owned(),
        });
    }

    let name = format!("Device-{ip_address}");
    match db_state.register_device_internal(ip_address, &name) {
        Ok(device) => Some(DeviceContext {
            device_id: device.id,
            device_name: device.name,
            ip_address: ip_address.to_owned(),
        }),
        Err(error) => {
            log::warn!("Failed to register device {ip_address}: {error}");
            None
        }
    }
}
