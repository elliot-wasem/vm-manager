use crate::config::{Config, VMConfig};
use crate::utils::{get_file_from_image_name, run_shell_command, find_open_port, is_port_in_use};
use crate::{DEFAULT_HTTPS_PORT, DEFAULT_SSH_PORT};
use anyhow::Result;
use std::path::PathBuf;

pub struct QemuRunner {
    daemonize: bool,
    ssh_port: usize,
    https_port: usize,
    specified_ssh_port: bool,
    specified_https_port: bool,
    image: PathBuf,
    pid: Option<usize>,
    vm_config: Option<VMConfig>,
}

impl Default for QemuRunner {
    fn default() -> Self {
        Self {
            daemonize: true,
            ssh_port: find_open_port(DEFAULT_SSH_PORT),
            https_port: find_open_port(DEFAULT_HTTPS_PORT),
            specified_ssh_port: false,
            specified_https_port: false,
            image: PathBuf::from(""),
            pid: None,
            vm_config: None,
        }
    }
}

impl QemuRunner {
    pub fn new(
        ssh_port: usize,
        https_port: usize,
        image_name: &str,
        pid: Option<usize>,
        config: &Config,
    ) -> Self {
        Self {
            daemonize: true,
            ssh_port,
            https_port,
            specified_ssh_port: false,
            specified_https_port: false,
            image: if let Some(image) = get_file_from_image_name(image_name, config) {
                image
            } else {
                PathBuf::from("")
            },
            pid,
            vm_config: None,
        }
    }
    pub fn set_ssh_port(&mut self, port: usize) {
        self.ssh_port = port;
        self.specified_ssh_port = true;
    }
    pub fn set_https_port(&mut self, port: usize) {
        self.https_port = port;
        self.specified_https_port = true;
    }
    pub fn set_image_file(&mut self, image_file: PathBuf) {
        self.image = image_file;
    }
    pub fn set_daemonization_option(&mut self, should_daemonize: bool) {
        self.daemonize = should_daemonize;
    }
    pub fn ssh_port(&self) -> usize {
        self.ssh_port
    }
    pub fn https_port(&self) -> usize {
        self.https_port
    }
    pub fn add_vm_config(&mut self, config: &VMConfig) {
        self.vm_config = Some(config.clone());
    }
    pub fn image_name(&self) -> String {
        if let Some(fstem) = self.image.file_stem() {
            fstem.to_os_string().to_str().unwrap().to_owned()
        } else {
            String::from("Can't get image name")
        }
    }
    fn start_with_vm_config(&self, config: &Config) -> Result<(), String> {
        if let Some(vm_config) = &self.vm_config {

            let drive_args: String = if let Some(image_path) = get_file_from_image_name(vm_config.image_name(), config) {
format!("file={}", image_path.display())
            } else {
                return Err(format!("Unable to find image with name containing '{}' in directory '{}'", vm_config.image_name(), config.get_images_directory()));
            };

            let mut args: Vec<&str> = vec![
                // TODO: Make this configurable via config file?
                "qemu-system-x86_64",
                if vm_config.daemonize() {
                    "-daemonize"
                } else {
                    "-nographic"
                },
                "-drive",
                &drive_args,
            ];

            // if we are daemonizing, we want it to run under nohup
            if vm_config.daemonize() {
                args.insert(0, "nohup");
            }

            for option in vm_config.options() {
                // because we have the specific `daemonize` option,
                // we don't want to duplicate flags if possible.
                if !option.as_str().starts_with("-daemonize")
                    && !option.as_str().starts_with("-nographic")
                {
                    args.append(&mut option.get_opt_list().clone());
                }
            }

            run_shell_command(&args)?;

            Ok(())
        } else {
            Err("No VM config provided!".to_string())
        }
    }
    pub fn start(&self, config: &Config) -> Result<(), String> {
        if self.vm_config.is_some() {
            self.start_with_vm_config(config)
        } else {
            // check that ssh port is okay
            if self.specified_ssh_port && is_port_in_use(self.ssh_port) {
                return Err(format!("ERROR: You specified SSH port '{}', which is in use. Choose a different port, or don't specify and let the program choose.", self.ssh_port));
            }

            // check that https port is okay
            if self.specified_https_port && is_port_in_use(self.https_port) {
                return Err(format!("ERROR: You specified HTTPS port '{}', which is in use. Choose a different port, or don't specify and let the program choose.", self.ssh_port));
            }

            // check that image file exists
            if !self.image.is_file() {
                return Err(format!(
                    "ERROR: Selected image file '{:?}' does not exist!",
                    &*self.image
                ));
            }

            let daemonization_opt: &str = if self.daemonize {
                "-daemonize"
            } else {
                "-nographic"
            };
            let nic_args: String = format!(
                "user,model=virtio,hostfwd=tcp::{}-:22,hostfwd=tcp::{}-:443",
                self.ssh_port, self.https_port
            );

            let drive_args: String = format!("file={}", (*self.image).display());

            let mut args: Vec<&str> = vec![
                "qemu-system-x86_64",
                daemonization_opt,
                "-drive",
                &drive_args,
                "-m",
                "8G",
                "-smp",
                "4",
                "-accel",
                "kvm",
                "-accel",
                "tcg",
                "-cpu",
                "host",
                "-vnc",
                "none",
                "-nic",
                &nic_args,
            ];

            if self.daemonize {
                args.insert(0, "nohup");
            }

            run_shell_command(&args)?;
            Ok(())
        }
    }

    pub fn stop(&self) -> Result<(), String> {
        if let Some(pid) = self.pid {
            run_shell_command(&["kill", &format!("{}", pid)])?;
            Ok(())
        } else {
            Err("No PID provided; cannot stop VM!".to_string())
        }
    }
}
