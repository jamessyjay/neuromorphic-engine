# OOP Architecture


## [cite_start]1. Базовый Класс: `BaseNeuron` [cite: 1]
**Свойства (Properties):**
* [cite_start]`id`: Уникальный идентификатор узла[cite: 1].
* [cite_start]`size`: Размер сомы (по дефолту средний)[cite: 1].
* [cite_start]`resting_voltage`: Базовый заряд (по дефолту -70.0 mV)[cite: 1].
* [cite_start]`activation_threshold`: Порог выстрела (по дефолту -55.0 mV)[cite: 1].
* [cite_start]`spike_peak`: Пик спайка (по дефолту +40.0 mV)[cite: 1].
* [cite_start]`energy_ATP`: Текущий уровень энергии (от 0 до 100%)[cite: 1].
* [cite_start]`is_alive`: Булево значение (true/false)[cite: 1].
* [cite_start]`inputs_array[]`: Массив входящих соединений (дендриты)[cite: 1].
* [cite_start]`outputs_array[]`: Массив исходящих соединений (аксон -> телодендрия)[cite: 1].

**Методы (Methods):**
* [cite_start]`calculate_sum()`: Сложить все входящие плюсы и минусы[cite: 1].
* [cite_start]`fire_spike()`: Если сумма > activation_threshold, ебануть ток до spike_peak[cite: 1].
* [cite_start]`release_neurotransmitter(type, amount)`: Выплюнуть химию в синапс после спайка[cite: 1].
* [cite_start]`apply_cooldown()`: Заблокировать fire_spike() на 1-3 мс (рефрактерный период)[cite: 1].
* [cite_start]`die_gracefully()`: Запустить апоптоз, если energy_ATP == 0 или жесткий перегруз кальцием[cite: 1].

## 2. Дочерние Классы (Наследники)
* [cite_start]**`PyramidalNeuron`:** Крупный (Пирамида)[cite: 1]. [cite_start]Base: -70 мВ[cite: 1]. [cite_start]Trigger: -55 мВ[cite: 1]. [cite_start]Главный процессор[cite: 1]. [cite_start]Наследует всё от базы[cite: 1]. [cite_start]Имеет неебически сложный calculate_sum(), так как входов тысячи[cite: 1]. [cite_start]Выделяет Глутамат (возбуждает других)[cite: 1].
* [cite_start]**`PurkinjeCell`:** Огромный (2D-веер)[cite: 1]. [cite_start]Base: -70 мВ[cite: 1]. [cite_start]Trigger: -50 мВ[cite: 1]. [cite_start]Параллельный вычислитель [cite: 1][cite_start]. inputs_array увеличен до 200,000[cite: 1]. [cite_start]Переопределен метод стрельбы: может делать обычный fire_spike(), а может fire_complex_spike() (мощный сброс весов для обучения таймингам)[cite: 1].
* [cite_start]**`BipolarNeuron`:** Мелкий (Провод)[cite: 1]. [cite_start]Base: -70 мВ[cite: 1]. [cite_start]Trigger: -55 мВ[cite: 1]. [cite_start]Ретранслятор[cite: 1]. [cite_start]Строго 1 вход и 1 выход[cite: 1]. [cite_start]Метод calculate_sum() почти не нужен[cite: 1]. [cite_start]Тупо принял пакет -> передал пакет[cite: 1]. [cite_start]Максимальная скорость[cite: 1].
* [cite_start]**`Interneuron`:** Мелкий (Куст)[cite: 1]. [cite_start]Base: -70 мВ[cite: 1]. [cite_start]Trigger: -55 мВ[cite: 1]. [cite_start]Локальный тормоз[cite: 1]. [cite_start]Обитает только локально[cite: 1]. [cite_start]Всегда вызывает release_neurotransmitter(GABA), то есть раздает всем соседям минусы, чтобы отсекать лишний шум и гасить эпилепсию[cite: 1].
* [cite_start]**`PacemakerNeuron`:** Средний[cite: 1]. [cite_start]Base: Плавает[cite: 1]. [cite_start]Trigger: -40 мВ[cite: 1]. [cite_start]Генератор тактовой частоты[cite: 1]. [cite_start]Метод calculate_sum() игнорируется[cite: 1]. [cite_start]Встроен while(is_alive) луп, который сам постоянно поднимает заряд до -40 и стреляет[cite: 1]. [cite_start]Задает ритм всей сети[cite: 1].
* [cite_start]**`Photoreceptor`:** Мелкий (Колбочка)[cite: 1]. [cite_start]Base: -40 мВ[cite: 1]. [cite_start]Trigger: NULL[cite: 1]. [cite_start]Ебанутый аналоговый датчик[cite: 1]. [cite_start]Полностью переписан метод выстрела: fire_spike() ОТКЛЮЧЕН[cite: 1]. [cite_start]Вместо него работает smooth_dimmer()[cite: 1]. [cite_start]Пока темно — льет медиатор без остановки[cite: 1]. [cite_start]Как только падает свет — заряд падает до -65 мВ, и выброс химии стопается (инвертированная логика)[cite: 1].
* [cite_start]**`Nociceptor`:** Средний (Длинный)[cite: 1]. [cite_start]Base: -65 мВ[cite: 1]. [cite_start]Trigger: Динамический[cite: 1]. [cite_start]Датчик урона (боли)[cite: 1]. [cite_start]Свойство activation_threshold не статично[cite: 1]. [cite_start]При частом использовании порог снижается (например, до -60 мВ), и сенсор начинает спамить алертами от малейшего касания (эффект воспаления)[cite: 1].

## 3. Сервисные Классы (Демоны)
* [cite_start]**`Astrocyte`:** Garbage Collector (GC) & Resource Manager[cite: 1]. [cite_start]Мониторит синапсы[cite: 1]. [cite_start]Если видит дохуя лишнего Глутамата — вызывает метод clean_synapse()[cite: 1]. [cite_start]Также подкидывает энергию (energy_ATP += 10) активным нейронам[cite: 1].
* [cite_start]**`SchwannCell / Oligodendrocyte`:** Network Optimizer[cite: 1]. [cite_start]Оборачивает outputs_array (аксон) в миелин (изоленту)[cite: 1]. [cite_start]Меняет физику передачи: вместо медленного поканального вычисления включает teleport_signal(), ускоряя пакет данных в 100 раз[cite: 1].
* [cite_start]**`Microglia`:** Antivirus / Firewall[cite: 1]. [cite_start]Плавает по сети[cite: 1]. [cite_start]Находит мертвые объекты (is_alive == false), вызывает метод delete() и очищает память от мусора[cite: 1].