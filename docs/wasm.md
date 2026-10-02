# WebAssembly: сборка и взаимодействие с Web

Этот документ описывает сборку крейта `riscv-emu-wasm` и обмен данными между Rust-эмулятором и браузерным кодом из `web/index.html`.

## 1. Роль WASM-слоя

Крейт `wasm` — тонкая обёртка над `riscv-emu-core`. Он не реализует инструкции или устройства самостоятельно, а экспортирует браузеру управляемый объект `WasmRiscv` через `wasm-bindgen`.

Общая схема:

```text
web/index.html
      │ JavaScript API, Uint8Array, keyboard, timer
      ▼
web/pkg/riscv_emu_wasm.js
      │ wasm-bindgen glue
      ▼
web/pkg/riscv_emu_wasm_bg.wasm
      │ Rust calls
      ▼
riscv-emu-core::Cpu
```

JavaScript отвечает за получение файлов по HTTP, планирование кадров, клавиатуру и Canvas. Rust отвечает за состояние CPU, исполнение инструкций, память, ELF, системные вызовы и framebuffer.

## 2. Требования для сборки

Нужны:

- установленный Rust toolchain;
- цель `wasm32-unknown-unknown`;
- `wasm-pack`;
- `wasm-bindgen-cli`, обычно устанавливаемый автоматически `wasm-pack`;
- статический HTTP-сервер для запуска страницы.

Проверка инструментов:

```sh
rustc --version
cargo --version
wasm-pack --version
rustup target list --installed
```

Если WASM-цель отсутствует:

```sh
rustup target add wasm32-unknown-unknown
```

Установка `wasm-pack`:

```sh
cargo install wasm-pack
```

## 3. Сборка всего Rust workspace

Из корня проекта:

```sh
cargo test --workspace
cargo build --workspace
```

Эти команды проверяют нативную сборку ядра и WASM-обёртки. Они не создают готовый JavaScript-пакет для браузера.

## 4. Сборка браузерного WASM-пакета

Из папки `wasm`:

```sh
cd wasm
wasm-pack build \
  --target web \
  --release \
  --out-dir ../web/pkg
```

Назначение параметров:

| Параметр | Значение |
|---|---|
| `--target web` | Генерирует ES module, загружаемый напрямую браузером |
| `--release` | Использует оптимизированный Cargo release-профиль |
| `--out-dir ../web/pkg` | Помещает итоговые файлы рядом с frontend |

Release-профиль определяется в корневом `Cargo.toml`:

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
```

После Rust-компиляции `wasm-pack` запускает `wasm-bindgen`, а при наличии — `wasm-opt`.

## 5. Результат сборки

В `web/pkg` создаются:

| Файл | Назначение |
|---|---|
| `riscv_emu_wasm_bg.wasm` | Скомпилированный Rust-код и WASM memory |
| `riscv_emu_wasm.js` | JavaScript-обвязка, созданная `wasm-bindgen` |
| `riscv_emu_wasm.d.ts` | TypeScript API экспортируемых классов и функций |
| `riscv_emu_wasm_bg.wasm.d.ts` | Низкоуровневые TypeScript-описания WASM exports |
| `package.json` | Метаданные сгенерированного пакета |

Файлы в `web/pkg` генерируются автоматически. Их не следует редактировать вручную: изменения будут потеряны при следующей сборке.

## 6. Запуск Web frontend

Страницу нельзя надёжно открывать напрямую через `file://`, поскольку она загружает ES module, WASM, ELF и WAD отдельными HTTP-запросами.

Из папки `web`:

```sh
cd web
python3 -m http.server 8080
```

После этого открыть:

```text
http://localhost:8080/
```

Для текущего запуска в `web` должны находиться:

```text
web/
├── index.html
├── doom.elf
├── doom1.wad
└── pkg/
    ├── riscv_emu_wasm.js
    └── riscv_emu_wasm_bg.wasm
```

`doom1.wad` в текущем проекте содержит Freedoom Phase 2, несмотря на имя файла. Doom Generic сначала выбирает Doom II-совместимый режим, поэтому WAD должен содержать соответствующие lumps.

## 7. Инициализация WASM в браузере

Frontend импортирует сгенерированные exports:

```js
import init, { WasmRiscv } from './pkg/riscv_emu_wasm.js';
```

Затем инициализирует модуль:

```js
const wasm = await init();
const wasmMemory = wasm.memory;
const emu = new WasmRiscv();
```

`init()` загружает `riscv_emu_wasm_bg.wasm`, создаёт его instance и возвращает низкоуровневые exports. `WasmRiscv` — высокоуровневая JavaScript-обёртка над Rust-структурой.

Ссылку на `wasm.memory` нужно сохранить, потому что framebuffer читается непосредственно из линейной памяти WebAssembly без отдельной сериализации каждого пикселя.

## 8. Загрузка ELF и WAD

Frontend получает оба файла параллельно:

```js
const [elfResponse, wadResponse] = await Promise.all([
  fetch('./doom.elf'),
  fetch('./doom1.wad?v=2'),
]);
```

`ArrayBuffer` преобразуется в `Uint8Array` и передаётся в Rust:

```js
emu.load_elf(new Uint8Array(await elfResponse.arrayBuffer()));
emu.load_wad(new Uint8Array(await wadResponse.arrayBuffer()));
```

### Что происходит в Rust

`load_elf()`:

1. Проверяет формат ELF32 little-endian и архитектуру RISC-V.
2. Копирует `PT_LOAD`-сегменты в гостевую DRAM.
3. Обнуляет BSS.
4. Устанавливает `pc` в ELF entry point.
5. Вычисляет начало heap по концу загруженного образа.
6. Настраивает стек для rvcore/Doom memory layout.

`load_wad()` хранит WAD отдельно от 8 MiB гостевой DRAM. Системные вызовы `_open`, `_read` и `_lseek` предоставляют Doom виртуальный файловый доступ к этому массиву.

## 9. Экспортируемый API

Rust-структура помечена `#[wasm_bindgen]`:

```rust
#[wasm_bindgen]
pub struct WasmRiscv {
    cpu: Cpu,
}
```

Публичные методы:

| Метод | Направление | Назначение |
|---|---|---|
| `new()` | JS → Rust | Создать новый CPU |
| `load_elf(data)` | JS → Rust | Загрузить исполняемый ELF |
| `load_binary(address, data)` | JS → Rust | Загрузить flat binary |
| `load_wad(data)` | JS → Rust | Подключить WAD как виртуальный файл |
| `run(count)` | JS → Rust | Исполнить пакет инструкций |
| `pc()` | Rust → JS | Вернуть текущий PC |
| `reg(index)` | Rust → JS | Вернуть значение регистра |
| `framebuffer_ptr()` | Rust → JS | Вернуть указатель в WASM memory |
| `framebuffer_size()` | Rust → JS | Вернуть размер framebuffer |
| `uart_output()` | Rust → JS | Забрать и очистить UART-буфер |
| `push_key(code)` | JS → Rust | Передать код клавиши |
| `set_ticks_ms(ticks)` | JS → Rust | Обновить виртуальный таймер |
| `halted()` | Rust → JS | Проверить остановку гостевой программы |

## 10. Выполнение инструкций

В каждом animation frame JavaScript вызывает:

```js
emu.set_ticks_ms(now | 0);
emu.run(2_000_000);
```

Rust исполняет до двух миллионов инструкций либо останавливается на ошибке. Большой пакет уменьшает количество переходов JS↔WASM и ускоряет первоначальную загрузку Doom.

Слишком маленький пакет приводит к долгому появлению первого кадра. Слишком большой может надолго блокировать UI thread. Значение следует подбирать по производительности целевых устройств.

В будущем лучше заменить фиксированный пакет методом вроде `run_until_frame(max_instructions)`, который возвращается сразу после системного вызова `_render_frame`.

## 11. Передача framebuffer без копирования из Rust

Rust возвращает указатель и длину массива:

```js
const source = new Uint8ClampedArray(
  wasmMemory.buffer,
  emu.framebuffer_ptr(),
  emu.framebuffer_size(),
);
```

Это view поверх той же линейной памяти WASM. `wasm-bindgen` не копирует весь framebuffer при вызове метода.

### Важное правило о `memory.grow`

После роста WASM memory старый `wasmMemory.buffer` и созданные на нём typed arrays могут стать неактуальными. Поэтому frontend создаёт `Uint8ClampedArray` заново после каждого `run()`.

### Формат пикселей

Doom/rvcore формирует little-endian `XRGB8888`, то есть в памяти байты представлены как B, G, R, X. Canvas `ImageData` ожидает R, G, B, A.

Frontend выполняет преобразование:

```js
const pixels = source.slice();

for (let i = 0; i < pixels.length; i += 4) {
  const blue = pixels[i];
  pixels[i] = pixels[i + 2];
  pixels[i + 2] = blue;
  pixels[i + 3] = 255;
}

ctx.putImageData(new ImageData(pixels, 640, 400), 0, 0);
```

`slice()` создаёт отдельный массив, чтобы перестановка каналов не изменяла Rust framebuffer. Alpha обязательно устанавливается в `255`: X-байт от Doom равен нулю и без преобразования Canvas покажет полностью прозрачный кадр.

## 12. Клавиатура

JavaScript преобразует DOM `KeyboardEvent.key` в коды платформенного слоя Doom:

```js
const keys = {
  Enter: 1,
  Escape: 2,
  ArrowLeft: 3,
  ArrowRight: 4,
  ArrowUp: 5,
  ArrowDown: 6,
  Control: 7,
  ' ': 8,
  Shift: 9,
  Alt: 10,
};
```

При `keydown` и `keyup` вызывается:

```js
emu.push_key_event(key, event.type === 'keydown');
```

Rust записывает событие и в MMIO-очередь, и в совместимый с rvcore кольцевой буфер в конце гостевой DRAM.

Повторные браузерные `keydown` игнорируются, а при потере фокуса все удерживаемые клавиши отпускаются. Это предотвращает переполнение очереди и залипание управления.

## 13. Таймер

WebAssembly-код не обращается напрямую к `performance.now()`. Время передаёт frontend:

```js
function frame(now) {
  emu.set_ticks_ms(now | 0);
  // ...
}
```

Гостевая программа получает время через MMIO или системный вызов `_gettimeofday`. Такой подход отделяет ядро от браузерных API и позволяет тестировать его нативно.

## 14. UART и диагностика

Гостевой `_write` сохраняет вывод в Rust `String`. JavaScript периодически забирает его:

```js
const output = emu.uart_output();
if (output) {
  console.log(output);
}
```

`uart_output()` использует операцию take: после чтения внутренний буфер очищается. Сообщения запуска Doom поэтому появляются в консоли браузера один раз.

## 15. Ошибки и ловушки

Методы `load_elf()` и `run()` возвращают `Result<(), JsValue>`. Rust-ошибка форматируется через `Debug` и становится JavaScript exception.

Примеры:

```text
AccessFault(7438336)
Misaligned(7364569)
IllegalInstruction(123456)
Ecall(42)
```

`Trap::Halted` обрабатывается как нормальное завершение и не выбрасывается в JavaScript. Состояние можно проверить через `halted()`.

Frontend ловит остальные ошибки и показывает текст в элементе `#status`:

```js
try {
  emu.run(count);
} catch (error) {
  running = false;
  status.textContent = String(error);
  console.error(error);
}
```

## 16. Кеш браузера

После пересборки WASM браузер может сохранить старые `.js`, `.wasm` или WAD. Во время разработки рекомендуется:

- выполнять полную перезагрузку страницы без кеша;
- менять query-параметр ресурса при замене его содержимого;
- проверять время изменения и размер файла через HTTP headers.

Пример cache busting:

```js
fetch('./doom1.wad?v=2');
```

Для production лучше использовать имена файлов с content hash.

## 17. Проверка готовой сборки

Проверить наличие файлов:

```sh
ls -lh web/pkg/riscv_emu_wasm.js \
       web/pkg/riscv_emu_wasm_bg.wasm \
       web/doom.elf \
       web/doom1.wad
```

Проверить HTTP:

```sh
curl -I http://127.0.0.1:8080/
curl -I http://127.0.0.1:8080/doom.elf
curl -I http://127.0.0.1:8080/doom1.wad
curl -I http://127.0.0.1:8080/pkg/riscv_emu_wasm_bg.wasm
```

Все запросы должны возвращать `200 OK`. Для `.wasm` ожидается MIME type `application/wasm`.

## 18. Типичный цикл разработки

После изменения только `web/index.html` пересобирать WASM не требуется — достаточно обновить страницу.

После изменения `core/src` или `wasm/src`:

```sh
cargo test --workspace
cd wasm
wasm-pack build --target web --release --out-dir ../web/pkg
```

HTTP-сервер можно не перезапускать: Python раздаёт актуальное содержимое файлов с диска.

Если frontend продолжает использовать старую версию, выполнить reload без кеша или изменить query version импорта/ресурса.

## 19. Возможные улучшения интеграции

1. Добавить `run_until_frame()` и событие готовности кадра.
2. Перенести XRGB→RGBA в WebGL shader и избежать CPU-копии `source.slice()`.
3. Запустить эмулятор в Web Worker, чтобы не блокировать UI.
4. Использовать `SharedArrayBuffer` при корректных COOP/COEP headers.
5. Добавить key-up события и состояние удерживаемых клавиш.
6. Добавить аудиокольцо и вывод через Web Audio API.
7. Использовать content-hashed имена WASM и WAD для корректного кеширования.
8. Экспортировать статистику: число инструкций, FPS и время исполнения пакета.
