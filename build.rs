// build.rs

fn main() {
    let is_macos = cfg!(target_os = "macos");

    if is_macos {
        // macOS: Compile CUDA code as standard C++ for CPU testing
        cc::Build::new()
            .cpp(true)
            .file("cuda/base_neuron.cu")
            .flag("-x")
            .flag("c++")
            .compile("neuron_kernels");
            
        println!("cargo:rustc-link-lib=c++");
    } else {
        // Jetson / Linux: Compile using nvcc and link CUDA runtime
        cc::Build::new()
            .cuda(true)
            .file("cuda/base_neuron.cu")
            .flag("-gencode=arch=compute_87,code=sm_87") // Orin Nano arch
            .compile("neuron_kernels");
            
        println!("cargo:rustc-link-lib=cudart");
    }

    println!("cargo:rerun-if-changed=cuda/");
}
