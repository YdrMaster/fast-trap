fn main() {
    use std::{env, fs, path::PathBuf};

    #[cfg(feature = "m-mode")]
    const BASE_ADDRESS: u64 = 0x8000_0000;
    #[cfg(feature = "s-mode")]
    const BASE_ADDRESS: u64 = 0x8020_0000;

    let ld = &PathBuf::from(env::var("OUT_DIR").unwrap()).join("linker.ld");
    fs::write(
        ld,
        format!(
            "\
OUTPUT_ARCH(riscv)
ENTRY(_start)
SECTIONS {{
    . = {BASE_ADDRESS};
    .text : {{
        *(.text.entry)
        *(.text .text.*)
    }}
    .rodata : {{
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }}
    .data : {{
        *(.data .data.*)
        *(.sdata .sdata.*)
    }}
    .bss : ALIGN(8) {{
        *(.bss.uninit)
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        ebss = .;
    }}
}}"
        ),
    )
    .unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-link-arg=-T{}", ld.display());
}
