# Как были подготовлены Doom ELF и WAD

Этот документ фиксирует происхождение файлов `web/doom.elf` и `web/doom1.wad`, точные команды сборки, использованные инструменты и проблемы, обнаруженные при интеграции с эмулятором.

## 1. Что находится в `web`

| Файл | Содержимое | Размер | SHA-256 |
|---|---|---:|---|
| `web/doom.elf` | Doom Generic, собранный для RV32IM/ILP32 | 684 520 байт | `11af5773fd3f061dca30809b3e26083bf372b1adb7c738873e328a364d77e012` |
| `web/doom1.wad` | Freedoom Phase 2 из релиза 0.13.0 | 28 787 748 байт | `a8772e088847032510d97ba2312406a6998f21cbab44d4ff10696faa9c0ecd4b` |

Имя `doom1.wad` сохранено из первоначальной структуры frontend, но фактически файл содержит `freedoom2.wad`. Причина описана ниже.

## 2. Источники

### Doom Generic с RISC-V backend

Исходный код был загружен из форка:

```text
https://github.com/lalitshankarch/doomgeneric.git
```

Использованный commit:

```text
3d6ff5d2f9af84fa7f9093335e5e6d0547d94f05
```

Этот форк уже содержит:

- `doomgeneric/doomgeneric_rvdoom.c` — платформенный слой для rvcore;
- `doomgeneric/stubs.h` — системные вызовы через `ECALL`;
- `doomgeneric/constants.h` — memory layout, VRAM и клавиатурная очередь;
- `doomgeneric/link.ld` — linker script для ELF от виртуального адреса `0`;
- Makefile с настройками `RV32IM/ILP32`.

### Эталонный эмулятор rvcore

Для понимания ABI и поведения системных вызовов использовался проект:

```text
https://github.com/lalitshankarch/rvcore.git
```

Использованный commit:

```text
17ec215620e06ce9b30b9d9e2cee981ad65a68eb
```

Из rvcore были сверены:

- номера системных вызовов `0..15`;
- расположение стека и VRAM;
- формат клавиатурной очереди;
- алгоритм загрузки low-address ELF;
- формат framebuffer `XRGB8888`;
- поведение виртуального файла WAD.

### Игровые данные

Оригинальные коммерческие Doom WAD в проект не добавлялись. Вместо них использован свободный набор игровых данных Freedoom:

```text
https://github.com/freedoom/freedoom/releases/download/v0.13.0/freedoom-0.13.0.zip
```

Из архива извлечён файл:

```text
freedoom-0.13.0/freedoom2.wad
```

Freedoom — свободная замена оригинальных ресурсов Doom. Код движка и игровые данные являются разными компонентами и распространяются отдельно.

## 3. Toolchain

Для сборки использовался официальный бинарный пакет xPack GNU RISC-V Embedded GCC для macOS ARM64:

```text
https://github.com/xpack-dev-tools/riscv-none-elf-gcc-xpack/releases/download/v15.2.0-1/xpack-riscv-none-elf-gcc-15.2.0-1-darwin-arm64.tar.gz
```

Версия компилятора:

```text
riscv-none-elf-gcc (xPack GNU RISC-V Embedded GCC arm64) 15.2.0
```

xPack выбран потому, что он включает:

- GCC для `riscv-none-elf`;
- binutils (`ld`, `readelf`, `objdump` и другие);
- newlib;
- multilib для `rv32im/ilp32`.

Наличие newlib обязательно: Doom Generic использует `malloc`, `printf`, `fopen`, `fread`, `fseek`, строковые функции и другие части стандартной C-библиотеки.

## 4. Загрузка исходников

Команды, эквивалентные использованным:

```sh
git clone --depth 1 \
  https://github.com/lalitshankarch/doomgeneric.git \
  /private/tmp/doomgeneric-riscv

git clone --depth 1 \
  https://github.com/lalitshankarch/rvcore.git \
  /private/tmp/rvcore-source
```

Для строго воспроизводимой сборки после клонирования следует checkout указанного выше commit, а не полагаться на текущее состояние default branch.

## 5. Установка xPack GCC

Архив был загружен и распакован во временную директорию:

```sh
curl -L --fail \
  -o /private/tmp/xpack-riscv.tar.gz \
  https://github.com/xpack-dev-tools/riscv-none-elf-gcc-xpack/releases/download/v15.2.0-1/xpack-riscv-none-elf-gcc-15.2.0-1-darwin-arm64.tar.gz

mkdir -p /private/tmp/xpack-riscv

tar -xzf /private/tmp/xpack-riscv.tar.gz \
  -C /private/tmp/xpack-riscv \
  --strip-components=1
```

Проверка:

```sh
/private/tmp/xpack-riscv/bin/riscv-none-elf-gcc --version
/private/tmp/xpack-riscv/bin/riscv-none-elf-gcc -print-file-name=libc.a
```

Вторая команда должна вернуть полный путь к `libc.a`, а не просто строку `libc.a`.

## 6. Компиляция Doom в ELF

Сборка выполнена в папке `doomgeneric`:

```sh
cd /private/tmp/doomgeneric-riscv/doomgeneric

make clean

make V=1 \
  CC=/private/tmp/xpack-riscv/bin/riscv-none-elf-gcc
```

Makefile форка добавляет основные параметры:

```text
-nostartfiles
-O2
-march=rv32im
-mabi=ilp32
-DNORMALUNIX
-DLINUX
-DSNDSERV
-D_DEFAULT_SOURCE
-T link.ld
```

Значение параметров:

| Параметр | Назначение |
|---|---|
| `-march=rv32im` | 32-битная базовая ISA с расширением умножения и деления |
| `-mabi=ilp32` | 32-битные `int`, `long` и указатели |
| `-O2` | Оптимизация Doom без чрезмерного роста кода |
| `-nostartfiles` | Используется собственная `_start` из `doomgeneric_rvdoom.c` |
| `-T link.ld` | Линковка исполняемого образа от адреса `0` |

Результирующий файл имеет исходное имя:

```text
/private/tmp/doomgeneric-riscv/doomgeneric/doomgeneric
```

Он был скопирован в проект под понятным runtime-именем:

```sh
cp /private/tmp/doomgeneric-riscv/doomgeneric/doomgeneric \
   web/doom.elf
```

## 7. Проверка ELF

Заголовок и сегменты проверялись командой:

```sh
/private/tmp/xpack-riscv/bin/riscv-none-elf-readelf \
  -h -l web/doom.elf
```

Ключевые свойства собранного файла:

```text
Class:                  ELF32
Data:                   little endian
Machine:                RISC-V
Entry point:            0x3a2d8
LOAD virtual address:   0x00000000
LOAD file size:         0x8c9b4
LOAD memory size:       0xc868c
Flags:                  RWE
```

Образ содержит один основной `PT_LOAD`, совместимый с моделью rvcore. Разница между `p_memsz` и `p_filesz` — BSS, которую ELF-загрузчик эмулятора заполняет нулями.

## 8. Подготовка WAD

Freedoom был загружен и распакован следующими командами:

```sh
curl -L --fail \
  -o /private/tmp/freedoom.zip \
  https://github.com/freedoom/freedoom/releases/download/v0.13.0/freedoom-0.13.0.zip

unzip -j -o \
  /private/tmp/freedoom.zip \
  '*/freedoom2.wad' \
  -d /private/tmp/freedoom-wad

cp /private/tmp/freedoom-wad/freedoom2.wad \
   web/doom1.wad
```

## 9. Проблемы и решения

### 9.1 Homebrew GCC не содержал newlib

Сначала был установлен пакет:

```sh
brew install riscv64-elf-gcc
```

Он предоставил компилятор `riscv64-elf-gcc`, но команда:

```sh
riscv64-elf-gcc -print-file-name=libc.a
```

возвращала только `libc.a`. Это означает, что библиотека отсутствовала. Doom компилируется из большого количества C-файлов и зависит от newlib, поэтому одного GCC/binutils недостаточно.

Решение: использовать xPack GNU RISC-V Embedded GCC, в который newlib уже включена.

### 9.2 Имя компилятора отличалось от Makefile

Makefile ожидает `riscv64-unknown-elf-gcc`, а xPack устанавливает `riscv-none-elf-gcc`.

Решение: не менять исходный Makefile, а передать полный путь:

```sh
make CC=/private/tmp/xpack-riscv/bin/riscv-none-elf-gcc
```

### 9.3 ELF загружался по адресу `0`, а память ожидала `0x80000000`

Первоначальная реализация DRAM принимала только диапазон от `DRAM_BASE = 0x80000000`. Doom ELF слинкован от `0` в соответствии с rvcore.

Решение: добавить два адресных alias одного массива DRAM:

- `0x00000000..0x007fffff` для Doom/rvcore;
- `0x80000000..0x807fffff` для обычных bare-metal программ.

### 9.4 Системные вызовы не совпадали

Первоначально эмулятор использовал стандартные номера наподобие `write = 64`, `exit = 93`. Порт Doom вызывает приватный ABI rvcore с номерами `0..15`.

Например:

- `0` — `_sbrk`;
- `1` — `_open`;
- `2` — `_read`;
- `4` — `_lseek`;
- `6` — `_gettimeofday`;
- `8` — `_render_frame`;
- `15` — `_exit`.

Решение: реализовать оба ABI параллельно. WAD хранится отдельно от DRAM и доступен гостю как один виртуальный seekable-файл.

### 9.5 Не помещавшийся в DRAM WAD

Freedoom WAD имеет размер около 27 MiB, а DRAM эмулятора — 8 MiB. Копировать WAD целиком в гостевую память нельзя.

Решение: хранить WAD в `Cpu::wad` на стороне хоста. `_read` копирует в DRAM только запрошенный гостевой буфер, а `_lseek` изменяет `wad_position`.

### 9.6 `Misaligned(7364569)`

Во время запуска newlib обратилась к адресу `0x705fd9`. Первоначальная память запрещала невыравненные `u16/u32` чтения и записи.

Эталонный rvcore выполняет такие операции программно, а Doom/newlib рассчитывает на это поведение.

Решение:

- разрешить невыравненные обращения к данным;
- оставить обязательное четырёхбайтовое выравнивание только для `pc` и выборки инструкции.

### 9.7 `AccessFault(7438336)`

Адрес ошибки — `0x718000`. Это был не обычный выход за DRAM, а новое значение heap после `_sbrk`.

Heap первоначально начинался с жёстко заданного `0x100000`, хотя загруженный ELF фактически заканчивается на `0x0c868c`. Лишний зазор привёл к преждевременному столкновению heap со стеком.

Решение: ELF-загрузчик вычисляет максимальное значение `p_vaddr + p_memsz`, выравнивает его вверх и использует как начальный heap.

### 9.8 `W_GetNumForName: d_dm2ttl not found!`

Сначала использовался `freedoom1.wad` (Freedoom Phase 1). Виртуальный `_open` возвращает один и тот же WAD на любой путь, а Doom первым проверяет Doom II-совместимое имя. Движок выбрал режим Doom II и запросил lump `D_DM2TTL`, которого нет в Phase 1.

Решение: заменить данные на `freedoom2.wad` (Freedoom Phase 2), совместимый с Doom II. Чтобы не менять уже существующий frontend route, файл сохранён как `web/doom1.wad`.

### 9.9 Doom работал, но Canvas оставался пустым

Smoke-тест ядра показал, что кадр существует и содержит 768 000 ненулевых байт. Ошибка была в формате изображения.

Doom/rvcore выдаёт little-endian `XRGB8888`, то есть байты в памяти имеют порядок `B, G, R, X`. Canvas `ImageData` ожидает `R, G, B, A`. Поле X было равно нулю и интерпретировалось как `alpha = 0`, поэтому весь кадр становился прозрачным.

Решение в JavaScript:

1. поменять местами красный и синий каналы;
2. установить alpha каждого пикселя в `255`;
3. только после этого передать массив в `ImageData`.

### 9.10 Первый кадр появлялся слишком долго

Первоначально за `requestAnimationFrame` исполнялось 250 000 инструкций. До первого кадра Doom выполняет примерно 64 миллиона инструкций, поэтому загрузка визуально казалась зависшей.

Решение: увеличить пакет до 2 000 000 инструкций. В будущем правильнее добавить `run_until_frame(max_instructions)`, чтобы завершать пакет непосредственно после `_render_frame`.

## 10. Проверка результата

Браузерный запуск подтверждает, что:

- ELF загрузился;
- newlib и системные вызовы работают;
- WAD читается;
- Doom дошёл до рендера;
- framebuffer содержит изображение.

Проверка файлов через HTTP:

```sh
curl -I http://127.0.0.1:8080/doom.elf
curl -I http://127.0.0.1:8080/doom1.wad
```

Оба запроса должны вернуть `200 OK`.

## 11. Полное воспроизведение

Краткая последовательность:

```sh
# 1. Скачать исходники Doom Generic.
git clone https://github.com/lalitshankarch/doomgeneric.git

# 2. Скачать и распаковать xPack GCC с newlib.
# URL указан в разделе Toolchain.

# 3. Собрать RV32IM ELF.
cd doomgeneric/doomgeneric
make clean
make CC=/path/to/xpack/bin/riscv-none-elf-gcc

# 4. Скопировать результат.
cp doomgeneric /path/to/cpu-emulator/web/doom.elf

# 5. Скачать Freedoom 0.13.0 и извлечь freedoom2.wad.
# Сохранить его в web/doom1.wad.

# 6. Пересобрать WASM после изменений ядра.
cd /path/to/cpu-emulator/wasm
wasm-pack build --target web --release --out-dir ../web/pkg

# 7. Запустить frontend.
cd ../web
python3 -m http.server 8080
```

## 12. Лицензии и распространение

Перед публикацией или распространением проекта следует отдельно проверить лицензии:

- Doom Generic и исходный Doom engine;
- форк `lalitshankarch/doomgeneric`;
- rvcore;
- Freedoom и его media assets;
- xPack toolchain не должен включаться в репозиторий вместе с результатом сборки без необходимости.

Оригинальные `doom.wad`, `doom2.wad` и другие коммерческие IWAD в этот проект не копировались.
