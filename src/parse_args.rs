use clap::{Parser, Subcommand};

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Must specify at least -i/--image, where the argument given to
    /// -i/--image is a unique substring of a name output by 'vm-manager -l' or
    /// 'vm-manager --list-images'.
    Start,
    /// Must specify at least -i/--image, where the argument given to
    /// -i/--image is a unique substring of a name output by 'vm-manager -r' or
    /// 'vm-manager --list-running-vms'.
    Stop,
}

/// Manage your qemu VMs.
/// ---------------------------------------------------------------------------
/// Installation process
///     1. Ensure qemu is installed.
///     2. Create a directory '$HOME/.vm-manager/disk-images'.
///     3. Create another directory '$HOME/.vm-manager/disk-images/backups'.
///     4. Put .img files for active vms in '$HOME/.vm-manager/disk-images'.
///     5. Put backup .img files in '$HOME/.vm-manager/disk-images/backups'.
#[derive(Debug, Parser)]
#[clap(name = "vm-manager", arg_required_else_help = true, verbatim_doc_comment)]
pub struct Arguments {
    /// List backup images.
    #[clap(long, short = 'b')]
    pub list_backup_images: bool,

    /// Run in foreground. Default is to daemonize.
    #[clap(long, short = 'f')]
    pub foreground: bool,

    /// Specify the image file with which to start the container.
    #[clap(long, short = 'i')]
    pub image: Option<String>,

    /// List images
    #[clap(long, short = 'l')]
    pub list_images: bool,

    /// Specify host port to forward to container's port 22. If this is not
    /// specified, the program will find the next available port >= the default
    /// port.
    #[clap(long, short = 'p')]
    pub ssh_port: Option<usize>,

    /// Specify host port to forward to container's port 443. If this is not
    /// specified, the program will find the next available port >= the default
    /// port.
    #[clap(long, short = 's')]
    pub https_port: Option<usize>,

    /// List running VMs.
    #[clap(long, short = 'r')]
    pub list_running_vms: bool,

    /// Config file to use. Default is '$HOME/.vm-manager/config.yml'.
    #[clap(long, short = 'c')]
    pub config_file: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}
