fn main() {
    println!("BaseNeuron size: {}", std::mem::size_of::<neuromorphic_engine::engine::ffi::BaseNeuron>());
    println!("Compartment size: {}", std::mem::size_of::<neuromorphic_engine::engine::ffi::Compartment>());
    assert_eq!(std::mem::size_of::<neuromorphic_engine::engine::ffi::BaseNeuron>(), 256);
    assert_eq!(std::mem::size_of::<neuromorphic_engine::engine::ffi::Compartment>(), 8);
}
