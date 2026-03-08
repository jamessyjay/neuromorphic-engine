# Use Cases & Tick Flow Diagram

This scheme describes the rigid sequence of actions (Decision Tree) within a single node for 1 unit of time (Tick = 1 ms). The agent must implement the main `update_tick()` cycle strictly according to this logic.

```mermaid
graph TD
    %% Entry Point
    Start((Tick Start)) --> IsAlive{Is cell alive?}
    IsAlive -- No --> DeadEnd([Skip tick])
    
    %% Block 1: State and Cooldowns
    IsAlive -- Yes --> CooldownCheck{Cooldown State?}
    CooldownCheck -- Absolute (1ms) --> AbsoluteBlock[Charge blocked at -80mV] --> MetabolicPhase
    CooldownCheck -- Relative (3ms) --> Penalty[Raise Threshold to -40mV] --> InputPhase
    CooldownCheck -- Free --> InputPhase

    %% Block 2: Data Reception (Use Cases 2 and 3)
    InputPhase[Collect packets from Synapse buffer] --> CalcVolt[Summation: <br> (Glutamate - GABA) * Dopamine]
    CalcVolt --> EnvCheck[Request resources from Environment]

    %% Block 3: Metabolism (Use Case 1)
    EnvCheck --> ATPGen[ATP +1% <br> if Glucose present]
    ATPGen --> LeakPump[Leak Rate: <br> Pull charge to -70mV. <br> ATP -0.1%]
    AbsoluteBlock --> MetabolicPhase[Metabolism works in background] --> ATPGen
    LeakPump --> ApplyVoltage[Apply final charge <br> to current_voltage]

    %% Block 4: Spike Verification (Use Case 2)
    ApplyVoltage --> ThresholdCheck{current_voltage >= Threshold?}
    
    %% Idle or Inhibition path (Use Cases 1 and 3)
    ThresholdCheck -- No --> CalciumCool[Cooling: Calcium -1] --> TickEnd((Tick End))

    %% Spike Branch (Use Case 2)
    ThresholdCheck -- Yes --> SpikeAction[SPIKE! <br> Charge = +40mV]
    SpikeAction --> ATPDrop[ATP -20.0%]
    ATPDrop --> StartCD[Start Absolute Cooldown]
    StartCD --> SendChem[Send transmitter <br> to outgoing Synapses]

    %% Block 5: Plasticity and Memory (Use Case 4)
    SendChem --> NMDACheck{Was Glutamate high <br> just before Spike?}
    NMDACheck -- Yes --> LTP[NMDA Open! <br> update_weights: Input weight +0.01]
    NMDACheck -- No --> CalciumSpike[Accumulate damage: <br> Calcium +10]
    LTP --> CalciumSpike

    %% Block 6: Death Check (Use Case 5)
    CalciumSpike --> DeathCheck{Calcium >= 100 <br> OR <br> ATP == 0?}
    DeathCheck -- Yes --> Die[die_gracefully: <br> Apoptosis / Shutdown]
    Die --> TickEnd
    DeathCheck -- No --> TickEnd
```