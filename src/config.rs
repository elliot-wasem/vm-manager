use serde::{Serialize, Deserialize};
use serde_yaml;

#[derive(Debug, Serialize, Deserialize)]
pub struct VMConfig {
    image_name: String,
    port_mappings: Vec<PortMapping>,
    options: Vec<QemuRunOption>,
    daemonize: bool,
}

/// This struct is used to represent a host-to-vm port mapping.
/// # Attributes:
/// * `host_port` - a `String` used to represent the port on the host to use.
/// * `vm_port` - a `String` used to represent the port on the vm to use.
/// * `explicit` - A boolean representing whether or not the exact specified
///     host port should be used. If 'true', then if that port is in use,
///     the program will exit. If 'false', then the program will find the next
///     highest available port.
#[derive(Debug, Serialize, Deserialize)]
pub struct PortMapping {
    /// Host port to be used.
    host_port: String,
    // VM port to which to bind
    vm_port: String,
    /// Whether or not the exact specified host port should be used.
    /// If 'true', then if that port is in use, the program
    /// will exit. If 'false', then the program will find the
    /// next highest available port.
    explicit: bool,
}

impl PortMapping {
    pub fn new_inexplicit(host_port: &str, vm_port: &str) -> Self {
        Self {
            host_port: host_port.to_owned(),
            vm_port: vm_port.to_owned(),
            explicit: false,
        }
    }
    pub fn new_explicit(host_port: &str, vm_port: &str) -> Self {
        Self {
            host_port: host_port.to_owned(),
            vm_port: vm_port.to_owned(),
            explicit: true,
        }
    }
    pub fn new(host_port: &str, vm_port: &str, explicit: bool) -> Self {
        Self {
            host_port: host_port.to_owned(),
            vm_port: vm_port.to_owned(),
            explicit,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QemuRunOption {
    option: String,
}

impl QemuRunOption {
    pub fn new(option: &str) -> Self {
        Self {
            option: option.to_owned(),
        }
    }
    pub fn as_str(&self) -> &str {
        &self.option
    }
    pub fn to_string(&self) -> String {
        self.option.clone()
    }
}
