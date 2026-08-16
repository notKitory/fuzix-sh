# fuzix (CLI)

[English](./README.md) | [Русский](./README_RU.md)

`fuzix` — современная нативная консольная утилита и SDK для разработки, компиляции и тестирования приложений под операционную систему **FUZIX OS**.

🚀 **100% Native & No Docker:** Работает напрямую на хосте (macOS Apple Silicon / Intel, Linux, Windows) без необходимости запускать Docker.  
🎮 **Вся мощь EmulatorKit:** Поддержка десятков плат и процессоров (Motorola 68000, Z80, Z180, 6809, 6502, 8080/8085 и др.).  
⚡ **Мгновенная скорость:** Нативный тулчейн компиляции и PTY-автоматизация эмуляторов.

---

## 📦 Установка

### Из исходников (Cargo)
```bash
cargo build --release
# Бинарник появится в target/release/fuzix
```

---

## 🛠 Быстрый старт

### 1. Создание нового проекта
```bash
fuzix init --name my-app --cpu 68000 --emulator v68
```
Команда создаст файл конфигурации `fuzix.toml` и шаблонный исходник `hello.c`.

### 2. Сборка C-программы
```bash
fuzix build hello.c
```

### 3. Автоматизированное тестирование (Сборка + Инжект + Запуск)
```bash
fuzix test hello.c arg1 arg2
```

### 4. Интерактивная консоль (FUZIX Shell)
```bash
fuzix shell
```
*Нажмите `Ctrl-]` или введите `shutdown` для завершения работы эмулятора.*

---

## 📋 Список команд

| Команда | Описание |
| --- | --- |
| `fuzix init` | Инициализирует новый FUZIX-проект и создает `fuzix.toml`. |
| `fuzix build [source.c]` | Компилирует C-файл или запускает `make` в нативном FUZIX тулчейне. |
| `fuzix test [source.c]` | Автоматически собирает C-файл, копирует его на диск и выполняет тест. |
| `fuzix run <cmd> [args...]` | Запускает команду внутри эмулятора FUZIX и возвращает вывод. |
| `fuzix shell` | Открывает интерактивную сессию терминала с эмулятором через PTY. |
| `fuzix disk cp <host> <fuzix>` | Записывает локальный файл на образ диска FUZIX (через `ucp`). |
| `fuzix disk ls [path]` | Просматривает файлы на диске FUZIX (по умолчанию `/bin`). |
| `fuzix emulators` | Выводит список поддерживаемых эмуляторов и плат из EmulatorKit. |

---

## ⚙️ Конфигурация (`fuzix.toml`)

Все настройки проекта хранятся в файле `fuzix.toml` в корне:

```toml
[project]
name = "my-app"
version = "0.1.0"
source = "hello.c"

[target]
cpu = "68000"          # 68000, z80, 8080, 6809, 6502
emulator = "v68"        # v68, tiny68k, rc2014, cpmsim, swt6809, rcbus-6502
timeout = 45

[disk]
boot_image = ".fuzix/images/boot.dsk"
root_image = ".fuzix/images/hd-fuzix.dsk"

[toolchain]
repo = "notKitory/fuzix-sh"
release = "latest"
```

---

## 🕹 Поддерживаемые эмуляторы (EmulatorKit & Virtual68)

Просмотреть все доступные системы можно командой `fuzix emulators`:

*   **`v68`** — Motorola 68000 с поддержкой IDE дисков и программного MMU *(Рекомендуется для 68k)*.
*   **`tiny68k`** / **`mini68k`** — одноплатные компьютеры на 68000.
*   **`rc2014`** / **`rcbus-z80`** — модульные компьютеры на Z80.
*   **`rcbus-z180`** — высокоскоростная система на Z180.
*   **`cpmsim`** — эмулятор 8080 / Z80 (z80pack).
*   **`swt6809`** — система на Motorola 6809.
*   **`rcbus-6502`** — система на MOS 6502.
*   **`altair8080`** — легендарный MITS Altair 8800.

---

## 🤝 Разработка

Будем рады вашим предложениям и Pull Request'ам в [GitHub репозитории](https://github.com/notKitory/fuzix-sh)!
