use serde::{Deserialize, Serialize};
use std::fs;

use crate::{IMAGES_DIRECTORY, utils::find_open_port};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
/// Used for storing a deserialized configuration.
/// # Attributes:
/// * base_images_directory - An `Option<String>` representing an image storage
///     directory. If `None`, uses `~/.vm-manager/disk-images` instead.
/// * global_qemu_options - A `Vec<QemuRunOption>` representing all qemu
///     options put in the `global_qemu_options` section.
/// * vms - A `Vec<VMConfig>` which holds the configuration options for
///     individual VMs.
pub struct Config {
    base_images_directory: Option<String>,
    global_qemu_options: Vec<QemuRunOption>,
    vms: Vec<VMConfig>,
}

impl Config {
    pub fn load_from_file(filename: &str) -> Self {
        // read config file to string
        let config_file_contents: String =
            fs::read_to_string(shellexpand::tilde(filename).to_string()).unwrap();

        // deserialize file contents to structured data
        let mut config: Self = match serde_yaml::from_str::<Self>(&config_file_contents) {
            Ok(config) => config,
            Err(e) => panic!("Unable to deserialize config file '{filename}'. {e}"),
        };

        // apply all global configs to each VM
        for vm in &mut config.vms {
            // push each option into the VM
            if vm.use_global_options {
                for option in &config.global_qemu_options {
                    vm.add_qemu_option(option);
                }
            }

            // check if `-nic` is present anywhere in options. If not, add it,
            // but only if there is at least one port mapping to add to it.
            if !vm.option_nic_present() && !vm.port_mappings.is_empty() {
                vm.add_qemu_option(&QemuRunOption::new("-nic"));
            }

            // add any port mappings to the `-nic` option.
            for option in &mut vm.options {
                if option.as_str().starts_with("-nic") {
                    let port_mappings: String = vm
                        .port_mappings
                        .iter_mut()
                        .map(|port_mapping| port_mapping.format_nic())
                        .collect::<Vec<String>>()
                        .join(",");
                    if option.as_str() == "-nic" {
                        // there are no other arguments to `-nic`, so any ports will be
                        // the only arguments.
                        option.option = format!("{} {}", &option.option, port_mappings,);
                    } else {
                        // there are other arguments to `-nic`, so any ports will be
                        // appended to other arguments.
                        option.option = format!("{},{}", &option.option, port_mappings,)
                    }
                }
            }
        }

        // if no base images directory was passed, use the program default.
        if config.base_images_directory.is_none() {
            config.base_images_directory = Some(IMAGES_DIRECTORY.to_owned());
        }

        config
    }

    pub fn get_images_directory(&self) -> String {
        //! Returns the specified images directory if
        //! `self.base_images_directory` is not `None`. If it IS `None`, then
        //! the directory `~/.vm-manager/disk-images` is used instead.
        if let Some(directory) = &self.base_images_directory {
            directory.to_owned()
        } else {
            IMAGES_DIRECTORY.to_owned()
        }
    }

    pub fn get_backup_images_directory(&self) -> String {
        //! Returns the specified backup images directory if
        //! `self.base_images_directory` is not `None`. This is located at
        //! `format!("{}/backup", self.get_images_directory())`. If it IS
        //! `None`, then the directory `~/.vm-manager/disk-images/backups` is
        //! used instead.
        format!("{}/backups", self.get_images_directory())
    }

    pub fn get_vm_config_with_image_name(&self, image_name: &str) -> Option<&VMConfig> {
        //! Searches through the list of VMs in `self.vms`, and returns either
        //! Some(vm) if the VM's image name contains the specified
        //! `image_name`, and `None` otherwise.
        self.vms.iter().find(|vm| vm.image_name().contains(image_name))
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct VMConfig {
    /// Name of the image to use, as shown in `$ vm-manager -l`.
    image_name: String,
    /// List of port mappings to apply. When passed to `qemu-system`, each will look like:
    /// ```
    ///     hostfwd=tcp::host_port-:vm_port
    /// ```
    port_mappings: Vec<PortMapping>,
    /// List of any arbitrary options to pass to `qemu-system`. Any `nic` options will be merged
    /// with `port_mappings`.
    options: Vec<QemuRunOption>,
    use_global_options: bool,
    daemonize: bool,
}

impl VMConfig {
    pub fn option_nic_present(&self) -> bool {
        //! Returns `true` if there is an option `-nic ...` present, and false otherwise.
        for option in &self.options {
            if option.option.starts_with("-nic") {
                return true;
            }
        }
        false
    }

    pub fn add_qemu_option(&mut self, option: &QemuRunOption) {
        //! Adds an option to the list of qemu options. Takes special care to avoid duplicate `-nic
        //! ...` options, and instead combines them.

        // we only want one `-nic` option, and the default behavior
        // is to overwrite when requesting to add one.
        //
        // first, check that the new option is a `-nic` option.
        if option.option.starts_with("-nic") && self.option_nic_present() {
            // next, iterate all options in self
            for self_option in &mut self.options {
                // if this option is a `-nic` option, we want to replace the contents
                // with the new option.
                if self_option.option.starts_with("-nic") {
                    self_option.option = option.option.clone();
                }
            }
        } else {
            self.options.push(option.clone());
        }
    }

    pub fn daemonize(&self) -> bool {
        self.daemonize
    }

    pub fn image_name(&self) -> &str {
        &self.image_name
    }

    pub fn options(&self) -> &Vec<QemuRunOption> {
        &self.options
    }
}

/// This struct is used to represent a host-to-vm port mapping.
/// # Attributes:
/// * `host_port` - a `String` used to represent the port on the host to use.
/// * `vm_port` - a `String` used to represent the port on the vm to use.
/// * `explicit` - A boolean representing whether or not the exact specified
///     host port should be used. If 'true', then if that port is in use,
///     the program will exit. If 'false', then the program will find the next
///     highest available port.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct PortMapping {
    /// Host port to be used.
    host_port: String,
    /// VM port to which to bind
    vm_port: String,
    /// Whether or not the exact specified host port should be used.
    /// If 'true', then if that port is in use, the program
    /// will exit. If 'false', then the program will find the
    /// next highest available port.
    explicit: bool,
}

impl PortMapping {
    #[allow(unused)]
    pub fn new(host_port: &str, vm_port: &str, explicit: bool) -> Self {
        Self {
            host_port: host_port.to_owned(),
            vm_port: vm_port.to_owned(),
            explicit,
        }
    }

    pub fn _host_port(&self) -> &str {
        &self.host_port
    }

    pub fn _vm_port(&self) -> &str {
        &self.vm_port
    }

    pub fn is_explicit_mapping(&self) -> bool {
        self.explicit
    }

    pub fn format_nic(&mut self) -> String {
        if !self.is_explicit_mapping() {
            if let Ok(host_port) = self.host_port.parse::<usize>() {
                self.host_port = format!("{}", find_open_port(host_port));
            } else {
                panic!("ERROR: For intended mapping from host port '{}' to VM port '{}', unable to read host port '{}' as an unsigned integer.", self.host_port, self.vm_port, self.host_port);
            }
        }
        format!("hostfwd=tcp::{}-:{}", self.host_port, self.vm_port)
    }
}

/// A struct to hold one or more related qemu run options.
///
/// These will be something like `-m 8G`, `-daemonize`, etc.
///
/// # Attributes:
/// * option - String representing the provided option(s).
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct QemuRunOption {
    option: String,
}

impl QemuRunOption {
    pub fn new(option: &str) -> Self {
        //! Creates a new QemuRunOption, replacing any tab characters with space characters.
        //!
        //! Examples:
        //!
        //! ```
        //! let option: QemuRunOption = QemuRunOption::new("-m 8G");
        //! assert_eq!(option.as_str(), "-m 8G");
        //!
        //! let option: QemuRunOption = QemuRunOption::new("-m\t8G");
        //! assert_eq!(option.as_str(), "-m 8G");
        //! ```
        Self {
            option: option.replace('\t', " ").to_owned(),
        }
    }
    pub fn as_str(&self) -> &str {
        //! Returns a `str` reference of the internal `option` attribute.
        &self.option
    }
    #[allow(unused)]
    pub fn is_multi_opts(&self) -> bool {
        //! Returns `true` if the given option string contains a space, and `false` otherwise.
        //!
        //! Examples:
        //!
        //! ```
        //! // `-m` and `8G` are two related but separate options.
        //! let option: QemuRunOption = QemuRunOption::new("-m 8G");
        //! assert!(option.is_multi_opts());
        //!
        //! // `-daemonize` is a single option.
        //! let option: QemuRunOption = QemuRunOption::new("-daemonize");
        //! assert!(!option.is_multi_opts());
        //!
        //! // `-m` and `8G` are two related but separate options. Upon object
        //! // creation, the tab is replaced with a space.
        //! let option: QemuRunOption = QemuRunOption::new("-m\t8G");
        //! assert!(option.is_multi_opts());
        //! ```
        self.option.split(' ').count() > 1
    }
    pub fn get_opt_list(&self) -> Vec<&str> {
        //! Vectorizes tab- or space-separated options
        //!
        //! Example:
        //!
        //! ```
        //! let option: QemuRunOption = QemuRunOption::new("-m 8G");
        //! assert_eq!(option.get_opt_list(), vec!["-m", "8G"]);
        //! ```
        self.option.split(' ').collect::<Vec<&str>>()
    }
}

mod tests {
    #[allow(unused)]
    use crate::config::VMConfig;

    #[test]
    fn test_qemu_run_option_new() {
        let option: crate::config::QemuRunOption = crate::config::QemuRunOption::new("-m 8G");
        assert_eq!(option.as_str(), "-m 8G");

        let option: crate::config::QemuRunOption = crate::config::QemuRunOption::new("-m\t8G");
        assert_eq!(option.as_str(), "-m 8G");
    }
    #[test]
    fn test_qemu_run_option_get_opt_list() {
        let option: crate::config::QemuRunOption = crate::config::QemuRunOption::new("-m 8G");
        assert_eq!(option.get_opt_list(), vec!["-m", "8G"]);

        let option: crate::config::QemuRunOption = crate::config::QemuRunOption::new("-daemonize");
        assert_eq!(option.get_opt_list(), vec!["-daemonize"]);

        let option: crate::config::QemuRunOption = crate::config::QemuRunOption::new("-m\t8G");
        assert_eq!(option.get_opt_list(), vec!["-m", "8G"]);
    }

    #[test]
    fn test_qemu_run_option_is_multi_opts() {
        let option: crate::config::QemuRunOption = crate::config::QemuRunOption::new("-m 8G");
        assert!(option.is_multi_opts());
        let option: crate::config::QemuRunOption = crate::config::QemuRunOption::new("-daemonize");
        assert!(!option.is_multi_opts());
        let option: crate::config::QemuRunOption = crate::config::QemuRunOption::new("-m\t8G");
        assert!(option.is_multi_opts());
    }

    #[test]
    fn test_serialize_vm_config() {
        let config: crate::config::VMConfig = crate::config::VMConfig {
            image_name: String::from("some-image-name"),
            port_mappings: vec![
                crate::config::PortMapping::new("5555", "22", false),
                crate::config::PortMapping::new("8081", "443", true),
            ],
            options: vec![
                crate::config::QemuRunOption::new("-m 8G"),
                crate::config::QemuRunOption::new("-daemonize"),
            ],
            use_global_options: true,
            daemonize: false,
        };

        let serialized_config: String = match serde_yaml::to_string(&config) {
            Ok(ser) => ser,
            Err(e) => format!("Serialization failure: {e}"),
        };

        let expected_string: String = String::from("image_name: some-image-name\nport_mappings:\n- host_port: '5555'\n  vm_port: '22'\n  explicit: false\n- host_port: '8081'\n  vm_port: '443'\n  explicit: true\noptions:\n- option: -m 8G\n- option: -daemonize\nuse_global_options: true\ndaemonize: false\n");

        assert_eq!(serialized_config, expected_string);
    }
    #[test]
    fn test_deserialize_vm_config() {
        let source_string: &str = "image_name: some-image-name\nport_mappings:\n- host_port: '5555'\n  vm_port: '22'\n  explicit: false\n- host_port: '8081'\n  vm_port: '443'\n  explicit: true\noptions:\n- option: -m 8G\n- option: -daemonize\nuse_global_options: true\ndaemonize: false";

        let deserialized_config: crate::config::VMConfig =
            match serde_yaml::from_str::<crate::config::VMConfig>(source_string) {
                Ok(deser) => deser,
                Err(e) => crate::config::VMConfig {
                    image_name: format!("Failed to deserialize: {e}"),
                    port_mappings: vec![],
                    options: vec![],
                    use_global_options: true,
                    daemonize: false,
                },
            };

        let expected_config: crate::config::VMConfig = crate::config::VMConfig {
            image_name: String::from("some-image-name"),
            port_mappings: vec![
                crate::config::PortMapping::new("5555", "22", false),
                crate::config::PortMapping::new("8081", "443", true),
            ],
            options: vec![
                crate::config::QemuRunOption::new("-m 8G"),
                crate::config::QemuRunOption::new("-daemonize"),
            ],
            use_global_options: true,
            daemonize: false,
        };

        assert_eq!(deserialized_config, expected_config);
    }
}
