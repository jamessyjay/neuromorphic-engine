# Neuromorphic Engine: API & FFI Reference

Техническое описание интерфейсов взаимодействия между Rust и C++/CUDA.

## Rust Structures (ffi.rs)

Все структуры помечены `#[repr(C)]` для бинарной совместимости.

### `Environment`
Глобальные параметры симуляции.
```rust
pub struct Environment {
    pub global_oxygen_level: f32,  // 0.0 - 1.0
    pub dopamine_multiplier: f32,  // 1.0 baseline
    pub toxin_levels: f32,
    pub seed: u32,                 // Глобальный сид хаоса
}
```

### `BaseNeuron`
Основная структура клетки (должна совпадать с `base_neuron.h`).
- `current_voltage`: Текущий потенциал (mV).
- `energy_ATP`: Текущий запас энергии.
- `metabolic_wear`: Износ митохондрий (0.0 - 1.0).
- `is_alive`: Флаг жизни.

## C++ Orchestration Functions

### `initial_neuron`
Инициализирует нейрон с учетом типа и гетерогенности.
```cpp
void init_neuron(BaseNeuron* neuron, uint32_t id, NeuronType type, uint32_t seed);
```
- Использует `seed + id` для генерации уникальных параметров по Гауссу.

### `launch_tick_neurons`
Основной цикл симуляции.
```cpp
void launch_tick_neurons(
    BaseNeuron* neurons, 
    int count, 
    Synapse* synapses, 
    int syn_count, 
    Environment env, 
    uint32_t current_tick
);
```
- Запускает расчет метаболизма и проводимости.
- На GPU выполняется как два последовательных ядра с барьером.

## CUDA Kernels

### `tick_neurons_kernel`
- Обрабатывает: `update_metabolism`, `update_calcium`, `process_spike`.
- Сложность: O(1) на поток.

### `tick_synapses_kernel`
- Обрабатывает: `process_synapse` (включая STDP и доставку).
- Сложность: O(1) на поток.

## Rust Services (Orchestration)
Вызываются из `main.rs` на стороне CPU:
- `astrocyte_service`: Очистка токсинов, поддержка метаболизма.
- `microglia_service`: Удаление мертвых нейронов (Apoptosis Cleanup).
