# fuzix-sh

[English](./README.md) | [Русский](./README_RU.md)

`fuzix-sh` — небольшая shell-обертка для компиляции и запуска программ на C для FUZIX через z80pack.

## Требования

- Docker
- `curl`
- `tar`

## Использование

```bash
./fuzix.sh compile <source.c>
./fuzix.sh image
./fuzix.sh run [-v] [arg...]
./fuzix.sh shell
./fuzix.sh test [-v] <source.c> [arg...]
```

При первом запуске скрипт скачает бинарники в `.fuzix-sh/prebuilt/<arch>` и соберет Docker runtime image. Следующие запуски переиспользуют этот runtime.

## Команды

| Команда | Описание |
| --- | --- |
| `compile <source.c>` | Компилирует C-файл в `.fuzix-sh/bin/<source-name>` и делает его текущей программой. |
| `image` | Копирует текущую скомпилированную программу в `/bin` внутри FUZIX root-диска. |
| `run [-v] [arg...]` | Загружает FUZIX в z80pack, запускает текущую программу, печатает вывод программы и выключает эмулятор. |
| `shell` | Открывает интерактивный FUZIX shell. `Ctrl-]` принудительно выключает эмулятор. |
| `test [-v] <source.c> [arg...]` | Последовательно выполняет `compile`, `image` и `run`. |

Используйте `-v`, чтобы видеть вывод эмулятора, а не только вывод программы.

Аргументы программы передаются сразу после команды:

```bash
./fuzix.sh test hello.c arg1 arg2
```

## State-директория

По умолчанию все сгенерированные файлы лежат в `.fuzix-sh`:

```text
.fuzix-sh
├── bin
├── build
├── images
│   ├── boot.dsk
│   └── hd-fuzix.dsk
└── prebuilt
    └── <arch>
```

`hd-fuzix.dsk` — единый mutable root-диск. Каждый `image` записывает текущую программу в один и тот же disk image.

## Разработка

Если у вас есть идеи, предложения или исправления, буду благодарен любым [issue](https://github.com/notKitory/fuzix-sh/issues) или [pull request](https://github.com/notKitory/fuzix-sh/pulls).
