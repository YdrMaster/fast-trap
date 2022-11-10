fn main() {
    use std::{env, fs, path::PathBuf};

    #[cfg(feature = "m-mode")]
    let base_address = 0x8000_0000usize;
    #[cfg(feature = "s-mode")]
    let base_address = match env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap().as_str() {
        "32" => 0x8040_0000usize,
        "64" => 0x8020_0000usize,
        _ => unreachable!(),
    };

    let ld = &PathBuf::from(env::var("OUT_DIR").unwrap()).join("linker.ld");
    fs::write(
        ld,
        format!(
            "\
OUTPUT_ARCH(riscv)
ENTRY(_start)
SECTIONS {{
    . = {base_address};
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
