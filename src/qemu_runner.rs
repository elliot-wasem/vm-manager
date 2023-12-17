use crate::utils::{get_file_from_image_name, run_shell_command};
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
}

impl Default for QemuRunner {
    fn default() -> Self {
        Self {
            daemonize: true,
            ssh_port: Self::find_open_port(DEFAULT_SSH_PORT),
            https_port: Self::find_open_port(DEFAULT_HTTPS_PORT),
            specified_ssh_port: false,
            specified_https_port: false,
            image: PathBuf::from(""),
            pid: None,
        }
    }
}

impl QemuRunner {
    pub fn new(ssh_port: usize, https_port: usize, image_name: &str, pid: Option<usize>) -> Self {
        Self {
            daemonize: true,
            ssh_port,
            https_port,
            specified_ssh_port: false,
            specified_https_port: false,
            image: if let Some(image) = get_file_from_image_name(image_name) {
                image
            } else {
                PathBuf::from("")
            },
            pid,
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
    fn is_port_in_use(port: usize) -> bool {
        match run_shell_command(&["lsof", "-nP", &format!("-i:{port}")]) {
            Ok(output) => output.status.success(),
            Err(_) => true,
        }
    }
    fn find_open_port(starting_port: usize) -> usize {
        let mut selected_port: usize = starting_port;
        while Self::is_port_in_use(selected_port) {
            selected_port += 1;
        }
        selected_port
    }
    pub fn image_name(&self) -> String {
        if let Some(fstem) = self.image.file_stem() {
            fstem.to_os_string().to_str().unwrap().to_owned()
        } else {
            String::from("Can't get image name")
        }
    }
    pub fn start(&self) -> Result<(), String> {
        // check that ssh port is okay
        if self.specified_ssh_port && Self::is_port_in_use(self.ssh_port) {
            return Err(format!("ERROR: You specified SSH port '{}', which is in use. Choose a different port, or don't specify and let the program choose.", self.ssh_port));
        }

        // check that https port is okay
        if self.specified_https_port && Self::is_port_in_use(self.https_port) {
            return Err(format!("ERROR: You specified HTTPS port '{}', which is in use. Choose a different port, or don't specify and let the program choose.", self.ssh_port));
        }

        // check that image file exists
        if !self.image.is_file() {
            return Err(format!(
                "ERROR: Selected image file '{:?}' does not exist!",
                &*self.image
            ));
        }

        let daemonization_opt: &str = if self.daemonize { "-daemonize" } else { "" };
        let nohup_cmd: &str = if self.daemonize { "nohup" } else { "" };

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
            args.insert(0, nohup_cmd);
        }

        run_shell_command(&args)?;
        Ok(())
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
