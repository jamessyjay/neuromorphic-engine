# Use Cases & Tick Flow Diagram

Эта схема описывает жесткую последовательность действий (Decision Tree) внутри одного узла за 1 такт времени (Tick = 1 ms). Агент обязан реализовать главный цикл `update_tick()` строго по этой логике.

```mermaid
graph TD
    %% Точка входа
    Start((Начало Тика)) --> IsAlive{Клетка жива?}
    IsAlive -- Нет --> DeadEnd([Пропуск такта])
    
    %% Блок 1: Состояние и Кулдауны
    IsAlive -- Да --> CooldownCheck{Состояние Кулдауна?}
    CooldownCheck -- Absolute (1ms) --> AbsoluteBlock[Заряд заблокирован на -80mV] --> MetabolicPhase
    CooldownCheck -- Relative (3ms) --> Penalty[Задрать Threshold до -40mV] --> InputPhase
    CooldownCheck -- Свободен --> InputPhase

    %% Блок 2: Прием данных (Юзкейс 2 и 3)
    InputPhase[Забрать пакеты из буфера Synapse] --> CalcVolt[Суммация: <br> (Glutamate - GABA) * Dopamine]
    CalcVolt --> EnvCheck[Запрос ресурсов из Environment]

    %% Блок 3: Метаболизм (Юзкейс 1)
    EnvCheck --> ATPGen[АТФ +1% <br> если есть Глюкоза]
    ATPGen --> LeakPump[Leak Rate: <br> Тянуть заряд к -70mV. <br> АТФ -0.1%]
    AbsoluteBlock --> MetabolicPhase[Метаболизм работает фоном] --> ATPGen
    LeakPump --> ApplyVoltage[Применить итоговый заряд <br> к current_voltage]

    %% Блок 4: Проверка выстрела (Юзкейс 2)
    ApplyVoltage --> ThresholdCheck{current_voltage >= Threshold?}
    
    %% Ветка простоя или Торможения (Юзкейс 1 и 3)
    ThresholdCheck -- Нет --> CalciumCool[Остывание: Кальций -1] --> TickEnd((Конец Тика))

    %% Ветка Спайка (Юзкейс 2)
    ThresholdCheck -- Да --> SpikeAction[СПАЙК! <br> Заряд = +40mV]
    SpikeAction --> ATPDrop[АТФ -20.0%]
    ATPDrop --> StartCD[Запустить Absolute Cooldown]
    StartCD --> SendChem[Отправить медиатор <br> в исходящие Synapse]

    %% Блок 5: Пластичность и Память (Юзкейс 4)
    SendChem --> NMDACheck{Был сильный Глутамат <br> прямо перед Спайком?}
    NMDACheck -- Да --> LTP[NMDA открыт! <br> update_weights: Вес входа +0.01]
    NMDACheck -- Нет --> CalciumSpike[Накопление урона: <br> Кальций +10]
    LTP --> CalciumSpike

    %% Блок 6: Проверка на смерть (Юзкейс 5)
    CalciumSpike --> DeathCheck{Кальций >= 100 <br> ИЛИ <br> АТФ == 0?}
    DeathCheck -- Да --> Die[die_gracefully: <br> Апоптоз / Отключение]
    Die --> TickEnd
    DeathCheck -- Нет --> TickEnd
```