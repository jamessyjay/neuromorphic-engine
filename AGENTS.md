---
trigger: always_on
---

# AI Agent Development Guidelines (Neuromorphic Engine)

## [!] ANTI-HALLUCINATION POLICY (ZERO TOLERANCE)

ИИ-агентам свойственно додумывать логику. В этом проекте **ЭТО КАТЕГОРИЧЕСКИ ЗАПРЕЩЕНО**.

1. **NO GUESSING:** Если формулы нет в `./specs/`, ты НЕ ИМЕЕШЬ ПРАВА ее выдумывать. Ты обязан остановиться и спросить пользователя.
2. **CITE YOUR SOURCES:** Перед написанием математики укажи в комменте, из какого файла спецификации ты это взял.
3. **NO GHOST DEPENDENCIES:** Запрещено импортировать сторонние библиотеки без разрешения.

---

## LANGUAGE POLICY

- **Код, переменные, названия функций и классов** — СТРОГО НА АНГЛИЙСКОМ (English only).
- **Комментарии в коде и документация** — можно на русском для лучшего понимания контекста.
- **NO EMOJIS OR SPECIAL SYMBOLS:** Никаких эмодзи. Только чистый ASCII в коде.

---

## [!] CRITICALLY IMPORTANT: CODE REUSE

**ПЕРЕД НАПИСАНИЕМ ЛЮБОГО КОДА — ПРОВЕРЬ:**

1. **ЕСТЬ ЛИ УЖЕ ГОТОВАЯ ФУНКЦИЯ?** Используй поиск по репозиторию. НИКОГДА не пиши новый код, если решение уже существует.
2. **МОЖНО ЛИ РАСШИРИТЬ СУЩЕСТВУЮЩУЮ ФУНКЦИЮ?** Добавь параметр вместо копипасты.

---

## [!] THE HYBRID STACK & BIOLOGICAL MODELING

### 1. ТЕХНОЛОГИЧЕСКИЙ СТЕК

- **RUST (Host/CPU):** Оркестратор. Управляет тиками, памятью и `Environment`. Запрещено использовать `.unwrap()` без необходимости.
- **C++ / CUDA C (Device/GPU):** Массивно-параллельная математика. Только **Unified Memory** (`cudaMallocManaged`).

### 2. ГЕТЕРОГЕННОСТЬ И ХАОС (NEW)

В природе нет клонов.

- **Initialization:** При создании нейронов ЗАПРЕЩЕНО использовать одинаковые константы. Используй нормальное распределение (Gaussian) для `resting_voltage`, `threshold` и `atp_capacity`.
- **Random Seeds:** В ядрах CUDA используй `curand` для генерации уникальных параметров каждого нейрона.

### 3. ГОМЕОСТАТИЧЕСКАЯ ПЛАСТИЧНОСТЬ (EVOLUTION)

Нейроны обязаны адаптироваться под нагрузкой.

- **Threshold Scaling:** Реализуй динамическое изменение порога. Если нейрон стреляет слишком часто — порог должен расти (`activation_threshold` повышается).
- **Metabolic Stress:** При низком АТФ эффективность насосов должна падать.

---

## FUNCTION ARCHITECTURE (1 Function = 1 Action)

**[!] ПРАВИЛЬНО (Atomic & Orchestrator):**

```cpp
// Atomic: According to 11_heterogeneity_and_validation.md
__device__ void apply_homeostatic_scaling(BaseNeuron* n) { /* Logic only */ }

// Orchestrator
__global__ void tick_neurons(BaseNeuron* neurons, int count) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= count || !neurons[idx].is_alive) return;
    apply_homeostatic_scaling(&neurons[idx]);
}
[!] TESTING & VALIDATION (DIGITAL TWIN)
Любая новая фича должна проходить через Stress-Tests (Спека 11):

Synaptic Fatigue: Проверка затухания сигнала при истощении везикул.

Hypoxic Shutdown: Проверка корректного отключения при oxygen_level -> 0.

Epileptic Feedback: Проверка срабатывания Апоптоза при саморазгоне сети.

VALIDATION: Сравнивай выходные графики (Voltage/Time) с биологическими эталонами (Allen Brain Institute). Форма спайка должна иметь фазу гиперполяризации.

DEVELOPMENT PROCESS
Task Analysis: Я понимаю задачу? Какая формула из ./specs/ нужна? Если нет — ASK!

Search for existing code: Проверь папки src/ и cuda/ на наличие готовых инструментов.

Planning (FFI): Опиши .h заголовки и Rust extern "C" обертку. Убедись, что типы f32 (Rust) и float (C++) совпадают.

Implementation: Пиши атомарные функции. Соблюдай SOLID.

Verification: Проверь код на Warp Divergence в CUDA и Memory Safety в Rust.

## [!] HARDWARE TELEMETRY & SCALING
Для предотвращения "светопредставления" при масштабировании:
1. **Telemetry Service:** В `graph.rs` должен быть модуль, который каждые 1000 тиков выводит:
   - `Simulation Speed`: сколько % от реального времени (например, 1.0x — идем в ногу со временем).
   - `Memory Overhead`: сколько байт тратится на 1 нейрон и 1 синапс в Unified Memory.
2. **GPU Profiling:** При работе на Jetson код должен уметь отдавать данные о GPU Utilization (через NVML).

[!] ЧЕКЛИСТ ПЕРЕД ВЫДАЧЕЙ КОДА
[ ] Прочитал ли я соответствующий файл из ./specs/?
[ ] Добавил ли я рандомизацию параметров (гетерогенность) при инициализации?
[ ] Реализована ли динамическая адаптация (гомеостаз) параметров?
[ ] Используется ли Unified Memory без локальных аллокаций в ядрах?
[ ] Разделена ли логика по SRP (1 функция = 1 действие)?
[ ] Код на английском, комментарии на русском?
[ ] Ссылаюсь ли я на файлы спецификаций в комментариях?
[ ] Внёс ли я информацию об изменениях в ./docs/walkthrough.md?
[ ] Протестировал ли я код на требуемом сценарии? Работает ли код?

ЕСЛИ ХОТЬ ОДИН ПУНКТ НЕ ВЫПОЛНЕН — ПЕРЕПИШИ КОД.
