#[macro_use]
extern crate clap;

use clap::Parser;
use once_cell::sync::Lazy;
use os_xtask_utils::{BinUtil, Cargo, CommandExt, Qemu};
use std::{
    fs,
    path::{Path, PathBuf},
};

static PROJECT: Lazy<&'static Path> =
    Lazy::new(|| Path::new(std::env!("CARGO_MANIFEST_DIR")).parent().unwrap());

#[derive(Parser)]
#[clap(name = "try-rtos")]
#[clap(version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Make(BuildArgs),
    Asm(AsmArgs),
    Qemu(QemuArgs),
}

fn main() {
    use Commands::*;
    match Cli::parse().command {
        Make(args) => {
            args.make();
        }
        Asm(args) => args.dump(),
        Qemu(args) => args.run(),
    }
}

#[derive(Args, Default)]
struct BuildArgs {
    /// Which mode, m or s
    #[clap(long)]
    mode: Option<char>,
    /// log level
    #[clap(long)]
    log: Option<String>,
    /// build in debug mode
    #[clap(long)]
    debug: bool,
}

impl BuildArgs {
    fn make(&self) -> PathBuf {
        let package = "test-app";
        let target = "riscv64imac-unknown-none-elf";
        Cargo::build()
            .package(package)
            .features(
                true,
                match self.mode {
                    Some('s') | None => ["s-mode"],
                    Some('m') => ["m-mode"],
                    Some(_) => panic!(),
                },
            )
            .optional(&self.log, |cargo, log| {
                cargo.env("LOG", log);
            })
            .conditional(!self.debug, |cargo| {
                cargo.release();
            })
            .target(target)
            .invoke();
        PROJECT
            .join("target")
            .join(target)
            .join(if self.debug { "debug" } else { "release" })
            .join(package)
    }
}

#[derive(Args)]
struct AsmArgs {
    #[clap(flatten)]
    build: BuildArgs,
    /// Output file.
    #[clap(short, long)]
    output: Option<String>,
}

impl AsmArgs {
    fn dump(self) {
        let elf = self.build.make();
        let out = PROJECT.join(self.output.unwrap_or("test-app.asm".into()));
        println!("Asm file dumps to '{}'.", out.display());
        fs::write(out, BinUtil::objdump().arg(elf).arg("-d").output().stdout).unwrap();
    }
}

#[derive(Args)]
struct QemuArgs {
    #[clap(flatten)]
    build: BuildArgs,
    /// Port for gdb to connect. If set, qemu will block and wait gdb to connect.
    #[clap(long)]
    gdb: Option<u16>,
}

impl QemuArgs {
    fn run(self) {
        let elf = self.build.make();
        let mode = if self.build.mode == Some('s') {
            "-kernel"
        } else {
            "-bios"
        };
        Qemu::system("riscv64")
            .args(&["-machine", "virt"])
            .arg("-nographic")
            .arg(mode)
            .arg(objcopy(elf, true))
            .args(&["-serial", "mon:stdio"])
            .optional(&self.gdb, |qemu, gdb| {
                qemu.args(["-S", "-gdb", &format!("tcp::{gdb}")]);
            })
            .invoke();
    }
}

fn objcopy(elf: impl AsRef<Path>, binary: bool) -> PathBuf {
    let elf = elf.as_ref();
    let bin = elf.with_extension("bin");
    BinUtil::objcopy()
        .arg(elf)
        .arg("--strip-all")
        .conditional(binary, |binutil| {
            binutil.args(["-O", "binary"]);
        })
        .arg(&bin)
        .invoke();
    bin
}
