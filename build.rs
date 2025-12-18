// build.rs
use std::env;

fn main() {
    // Generate build info
    vergen::EmitBuilder::builder()
        .all_build()
        .all_git()
        .emit()
        .expect("Unable to generate build info");
    
    // Check if ebpf feature is enabled
    if env::var("CARGO_FEATURE_EBPF").is_ok() {
        compile_ebpf_programs();
    }
}

#[cfg(feature = "ebpf")]
fn compile_ebpf_programs() {
    use std::path::PathBuf;
    use std::process::Command;
    
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let bpf_src = PathBuf::from("src/ebpf/bpf");
    
    // Check for required tools
    check_tool("clang", "--version");
    check_tool("bpftool", "version");
    
    println!("cargo:rerun-if-changed=src/ebpf/bpf/process_io.bpf.c");
    
    // Generate vmlinux.h if needed
    let vmlinux_h = bpf_src.join("vmlinux.h");
    if !vmlinux_h.exists() {
        println!("cargo:warning=Generating vmlinux.h from kernel BTF...");
        let output = Command::new("bpftool")
            .args(&["btf", "dump", "file", "/sys/kernel/btf/vmlinux", "format", "c"])
            .current_dir(&bpf_src)
            .output()
            .expect("Failed to generate vmlinux.h");
        
        if !output.status.success() {
            panic!("Failed to generate vmlinux.h. BTF support required.");
        }
        
        // Validate that the output looks like a C header
        let output_str = String::from_utf8_lossy(&output.stdout);
        if !output_str.contains("#ifndef") || !output_str.contains("struct") {
            panic!("Generated vmlinux.h does not appear to be a valid C header");
        }
        
        std::fs::write(&vmlinux_h, output.stdout)
            .expect("Failed to write vmlinux.h");
    }
    
    // Compile eBPF program
    let bpf_obj = out_dir.join("process_io.bpf.o");
    let status = Command::new("clang")
        .args(&[
            "-g",
            "-O2",
            "-target", "bpf",
            "-D__TARGET_ARCH_x86",
            "-I", bpf_src.to_str().unwrap(),
            "-c", bpf_src.join("process_io.bpf.c").to_str().unwrap(),
            "-o", bpf_obj.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to compile eBPF program");
    
    if !status.success() {
        panic!("eBPF compilation failed");
    }
    
    println!("cargo:rustc-env=EBPF_OBJECT_PATH={}", bpf_obj.display());
    
    fn check_tool(tool: &str, arg: &str) {
        let output = Command::new(tool)
            .arg(arg)
            .output();
        
        match output {
            Ok(out) if out.status.success() => {
                println!("cargo:warning=Found {}: OK", tool);
            }
            _ => {
                panic!("{} not found or failed. Required for eBPF compilation.", tool);
            }
        }
    }
}

#[cfg(not(feature = "ebpf"))]
fn compile_ebpf_programs() {}
