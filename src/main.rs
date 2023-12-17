mod parse_args;
mod qemu_runner;
mod utils;

use crate::{
    qemu_runner::QemuRunner,
    utils::{
        get_file_from_image_name, get_list_of_images, get_list_of_running_vms,
        print_running_vm_table, OutputStream, OutputStreamTarget,
    },
};
use anyhow::Result;
use clap::Parser;
use parse_args::Arguments;

const DEFAULT_SSH_PORT: usize = 5555;
const DEFAULT_HTTPS_PORT: usize = 8081;
const IMAGES_DIRECTORY: &str = "~/qemu-disks";
const BACKUP_IMAGES_DIRECTORY: &str = "~/qemu-disks/backups";

/// Options for disk image location.
enum ImageLocation {
    ///refers to IMAGES_DIRECTORY
    WorkingImages,
    ///refers to BACKUP_IMAGES_DIRECTORY
    BackupImages,
}

fn main() {
    let args = Arguments::parse();

    // used for collecting string output
    let mut buffer: OutputStream = OutputStream::new(OutputStreamTarget::Stdout);

    if args.list_images {
        buffer.addln("--------------------\nImages\n--------------------");
        for file in get_list_of_images(ImageLocation::WorkingImages) {
            buffer.addln(&format!("{file}"));
        }
    }

    if args.list_backup_images {
        buffer.add_spacer();
        buffer.addln("--------------------\nBackup Images\n--------------------");
        for file in get_list_of_images(ImageLocation::BackupImages) {
            buffer.addln(&format!("{file}"));
        }
    }

    if args.list_running_vms {
        buffer.add_spacer();
        let running_vms = get_list_of_running_vms();
        if running_vms.is_empty() {
            buffer.addln("No machines running.");
        } else {
            buffer.addln("--------------------\nRunning VMs\n--------------------");
            print_running_vm_table(&running_vms, &mut buffer);
        }
    }

    let command_result = match args.command {
        Some(parse_args::Command::Start) => {
            run_command_start(args.image, args.ssh_port, args.https_port, args.foreground)
        }
        Some(parse_args::Command::Stop) => run_command_stop(args.image),
        _ => Ok(()),
    };

    if let Err(e) = command_result {
        match args.command {
            Some(parse_args::Command::Start) => {
                buffer.add_spacer();
                buffer.addln(&format!(
                    "{e}\n\n--------------------\nImages\n--------------------"
                ));
                for file in get_list_of_images(ImageLocation::WorkingImages) {
                    buffer.addln(&file);
                }
            }
            Some(parse_args::Command::Stop) => {
                buffer.add_spacer();
                buffer.addln(e.as_str());
                let running_vms: Vec<QemuRunner> = get_list_of_running_vms();
                if !running_vms.is_empty() {
                    buffer.addln("\n--------------------\nRunning VMs\n--------------------");
                    print_running_vm_table(&running_vms, &mut buffer);
                }
            }
            _ => (),
        }
        buffer.flush();
        std::process::exit(1)
    }
    buffer.flush();
}

fn run_command_start(
    image: Option<String>,
    ssh_port: Option<usize>,
    https_port: Option<usize>,
    foreground: bool,
) -> Result<(), String> {
    if let Some(image_name) = image {
        let mut runner: QemuRunner = QemuRunner::default();
        if let Some(pathbuf) = get_file_from_image_name(&image_name) {
            runner.set_image_file(pathbuf);
        } else {
            return Err(format!(
                "Could not find unique image matching '{}'.",
                image_name
            ));
        }
        if let Some(port) = ssh_port {
            runner.set_ssh_port(port);
        }
        if let Some(port) = https_port {
            runner.set_https_port(port);
        }
        runner.set_daemonization_option(!foreground);

        runner.start()?;
        Ok(())
    } else {
        Err("No image provided! Must provide an image name.".to_owned())
    }
}

fn run_command_stop(image: Option<String>) -> Result<(), String> {
    if get_list_of_running_vms().is_empty() {
        return Err("No VMs running.".to_owned());
    }

    if let Some(image_name) = image {
        let mut found_image: bool = false;
        for image in get_list_of_images(ImageLocation::WorkingImages) {
            if image.contains(&image_name) {
                found_image = true;
                break;
            }
        }
        if !found_image {
            Err("No VM running on image with name matching {image_name}.".to_string())
        } else {
            let vms: Vec<QemuRunner> = get_list_of_running_vms();
            for vm in vms {
                if vm.image_name().contains(&image_name) {
                    return vm.stop();
                }
            }
            Err(format!(
                "Could not find a VM running with image name matching pattern '{image_name}'."
            ))
        }
    } else {
        Err("No image provided! Must provide an image name.".to_owned())
    }
}
