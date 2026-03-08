# neuromorphic-engine: MacOS Local Environment Setup

Так как на Маке (Apple Silicon / Intel) нет ни ядер CUDA, ни Unified Memory от Nvidia (это фича Jetson Devices), мы используем подход заглушек.

## 1. Как работает код на Маке
Когда ты запускаешь проект на macOS:
* Скрипт `build.rs` детектит, что это яблоко.
* Он собирает CUDA-файл `cuda/base_neuron.cu` как обычный C++ код, просто используя системный компилятор (clang из Xcode).
* Макросы `__global__`, `__device__` и другие CUDA-специфичные штуки — скрываются через препроцессор: они пустые (`#define __global__`). 
* Вместо `cudaMallocManaged` используется дефолтный системный `malloc`.
* Логика ядра (например, расчет Leak Rate или Calcium) выполняется последовательно (в for-цикле) просто на CPU Мака.

Это позволяет проверять математику, работу FFI и логику оркестратора **не имея Jetson под рукой**.

## 2. Подготовка

**Шаг 1. Убедиться, что установлен Rust**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustc --version
```

**Шаг 2. Установить C++ компилятор (если еще нет)**
```bash
xcode-select --install
```

**Шаг 3. Установка инструментов сборки**
Для работы C/C++ оберток (`cc` в `build.rs`):
Обычно ничего дополнительного не нужно, `cc` сам найдет `clang` на Маке.

## 3. Запуск

Просто перейди в папку проекта `neuromorphic-engine` и набери:

```bash
cargo run
```

Она сама скомпилирует `base_neuron.cu` в `librust_kernels.a`, линканет с Rust и выведет в терминал тестовые тики. 

Если код падает с FFI-ошибками, убедись, что структуры `BaseNeuron` в `cuda/base_neuron.h` 1 в 1 совпадают с `BaseNeuron` в `src/ffi.rs` (порядок, размер и тип полей!).
