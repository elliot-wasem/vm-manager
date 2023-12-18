use crate::config::Config;
use crate::qemu_runner::QemuRunner;
use crate::{ImageLocation, IMAGES_DIRECTORY};
use anyhow::Result;
use std::cmp::max;
use std::fs::read_dir;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::str::Chars;

pub enum OutputStreamTarget {
    Stdout,
}

pub struct OutputStream {
    stream: OutputStreamTarget,
    buffer: String,
}

impl OutputStream {
    pub fn new(stream: OutputStreamTarget) -> Self {
        Self {
            buffer: String::new(),
            stream,
        }
    }
    pub fn add(&mut self, input: &str) {
        self.buffer.push_str(input);
    }
    pub fn addln(&mut self, input: &str) {
        if !self.buffer.is_empty() {
            self.add_spacer();
        }
        self.add(input);
    }
    pub fn add_spacer(&mut self) {
        if !self.buffer.is_empty() {
            self.buffer.push('\n');
        }
    }
    pub fn flush(&mut self) {
        if self.buffer.is_empty() {
            return;
        }
        match self.stream {
            OutputStreamTarget::Stdout => println!("{}", self.buffer),
        }
        self.buffer = String::new();
    }
}

pub fn run_shell_command(command: &[&str]) -> Result<Output, String> {
    //! Runs an arbitrary shell command, returning either the output object on success, or a reason
    //! on failure.
    //!
    //! Example:
    //! ```
    //! match run_shell_command(&["ls", "-ld"]) {
    //!     Ok(output) => (), // here you can do something with output.stdout,
    //!                       // or check return status with output.status etc.
    //!     Err(reason) => eprintln!("An error occurred: {reason}"),
    //! }
    //! ```
    match Command::new(command[0]).args(&command[1..]).output() {
        Ok(output) => Result::Ok(output),
        Err(e) => Err(e.to_string()),
    }
}

pub fn get_list_of_images(image_location: ImageLocation, config: &Config) -> Vec<String> {
    //! Returns a vector of image names found in the given location.
    //!
    //! If the provided location is ImageLocation::WorkingImages, then
    //! it will search the path ~/.vm-manager/disk-images
    //!
    //! If the provided location is ImageLocation::BackupImages, then
    //! it will search the path ~/.vm-manager/disk-images/backups
    let images_directory: String = match image_location {
        ImageLocation::WorkingImages => config.get_images_directory(),
        ImageLocation::BackupImages => config.get_backup_images_directory(),
    };

    match read_dir(shellexpand::tilde(&images_directory).to_string()) {
        Err(e) => {
            eprintln!("{e}");
            vec![]
        }
        Ok(iter) => iter
            .filter_map(|f| match f {
                Ok(file_entry) => Some(file_entry),
                Err(e) => {
                    eprintln!("{e}");
                    None
                }
            })
            .filter_map(|f| {
                if f.path().is_file() {
                    f.path().as_path().file_stem().map(|path| path.to_owned())
                } else {
                    None
                }
            })
            .filter_map(|f| {
                if let Some(filename) = f.to_os_string().to_str() {
                    if filename != "nohup" {
                        Some(filename.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect(),
    }
}

pub fn get_list_of_running_vms(config: &Config) -> Vec<QemuRunner> {
    let output: String = match run_shell_command(&["ps", "ax"]) {
        Ok(output) => match String::from_utf8(output.stdout) {
            Ok(stdout) => stdout,
            Err(e) => {
                println!("from_utf ERROR: {e:#?}");
                return vec![];
            }
        },
        Err(e) => {
            println!("run_shell_command ERROR: {e:#?}");
            return vec![];
        }
    };

    let mut result: Vec<QemuRunner> = vec![];

    for line in output
        .split('\n')
        .filter(|l| l.contains("qemu-system-x86_64"))
        .collect::<Vec<&str>>()
    {
        let strings: Vec<&str> = line.split_ascii_whitespace().collect();
        let mut filename: String = String::new();
        if let Some(fname) = strings[7].split('=').nth(1) {
            if let Some(fstem) = Path::new(fname)
                .file_stem()
                .unwrap()
                .to_os_string()
                .to_str()
            {
                filename = fstem.to_owned();
            }
        } else {
            continue;
        }

        let pid: usize = strings[0]
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("Unable to parse '{}' as usize.", strings[0]));
        let port_data: Vec<&str> = strings[21].split(',').collect();
        let ssh_port_data: Vec<&str> = port_data[2].split(':').collect();
        let https_port_data: Vec<&str> = port_data[3].split(':').collect();
        let mut ssh_port: Chars = ssh_port_data[2].chars();
        let mut https_port: Chars = https_port_data[2].chars();

        // remove trailing '-' character
        ssh_port.next_back();
        https_port.next_back();

        let running_vm_entry: QemuRunner = QemuRunner::new(
            ssh_port.as_str().parse::<usize>().unwrap(),
            https_port.as_str().parse::<usize>().unwrap(),
            &filename,
            Some(pid),
            config,
        );
        result.push(running_vm_entry);
    }

    result
}

pub fn print_running_vm_table(running_vms: &[QemuRunner], output_buffer: &mut OutputStream) {
    let image_name_header_len = "image name".len();
    let image_name_width: usize = if let Some(max_elem) =
        running_vms.iter().reduce(|last_max, elem| {
            if last_max.image_name().len() > elem.image_name().len() {
                last_max
            } else {
                elem
            }
        }) {
        max(max_elem.image_name().len(), image_name_header_len)
    } else {
        image_name_header_len
    } + 2;
    output_buffer.addln(&format!(
        "{:8} | {:10} | {:width$}",
        "SSH Port",
        "HTTPS Port",
        "Image Name",
        width = image_name_width
    ));
    output_buffer.addln(&format!(
        "{:-<8}-+-{:-<10}-+-{:-<width$}",
        "",
        "",
        "",
        width = image_name_width
    ));
    for vm in running_vms {
        output_buffer.addln(&format!(
            "{:-8} | {:-10} | {:-width$}",
            vm.ssh_port(),
            vm.https_port(),
            vm.image_name(),
            width = image_name_width
        ));
    }
}

pub fn get_file_from_image_name(image_name: &str, config: &Config) -> Option<PathBuf> {
    let mut num_found = 0;
    let mut real_image_name = String::new();
    for full_image_name in get_list_of_images(ImageLocation::WorkingImages, config) {
        if full_image_name.contains(image_name) {
            real_image_name = full_image_name;
            num_found += 1;
        }
    }

    if real_image_name.is_empty() || num_found > 1 {
        return None;
    }

    let proposed_path: PathBuf = PathBuf::from(
        shellexpand::tilde(&format!("{IMAGES_DIRECTORY}/{real_image_name}.img")).to_string(),
    );
    if !proposed_path.is_file() {
        None
    } else {
        Some(proposed_path.to_owned())
    }
}
pub fn is_port_in_use(port: usize) -> bool {
    match run_shell_command(&["lsof", "-nP", &format!("-i:{port}")]) {
        Ok(output) => output.status.success(),
        Err(_) => true,
    }
}
pub fn find_open_port(starting_port: usize) -> usize {
    let mut selected_port: usize = starting_port;
    while is_port_in_use(selected_port) {
        selected_port += 1;
    }
    selected_port
}
