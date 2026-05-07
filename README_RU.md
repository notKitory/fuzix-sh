# fuzix-sh

[English](./README.md) | [Русский](./README_RU.md)

`fuzix-sh` — небольшая shell-обертка для компиляции и запуска программ на C для FUZIX через z80pack.

## Требования

- Docker
- `curl`
- `tar`

## Использование

Для использования достаточно скачать только [`fuzix.sh`](https://raw.githubusercontent.com/notKitory/fuzix-sh/main/fuzix.sh).

```bash
./fuzix.sh compile <source.c>
./fuzix.sh cp <host-path> <fuzix-path>
./fuzix.sh make [target...]
./fuzix.sh run [-v] <command> [arg...]
./fuzix.sh shell
./fuzix.sh test [-v] <source.c> [arg...]
```

При первом запуске скрипт скачает бинарники в `.fuzix-sh/prebuilt/<arch>` и соберет Docker runtime image. Следующие запуски переиспользуют этот runtime.

## Команды

| Команда | Описание |
| --- | --- |
| `compile <source.c>` | Компилирует C-файл в `.fuzix-sh/bin/<source-name>`. |
| `cp <host-path> <fuzix-path>` | Копирует локальный файл в FUZIX root-диск по пути `<fuzix-path>`. |
| `make [target...]` | Запускает `make` в окружении FUZIX toolchain. |
| `run [-v] <command> [arg...]` | Загружает FUZIX в z80pack, выполняет команду в FUZIX shell, печатает вывод команды и выключает эмулятор. |
| `shell` | Открывает интерактивный FUZIX shell. |
| `test [-v] <source.c> [arg...]` | Последовательно выполняет `compile`, `cp` и `run`. |

Используйте `-v`, чтобы видеть вывод эмулятора, а не только вывод программы.
`Ctrl-]` принудительно выключает эмулятор в `run` и `shell`.

Аргументы команды передаются сразу после команды:

```bash
./fuzix.sh run ls /bin
./fuzix.sh run /bin/hello arg1 arg2
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

`hd-fuzix.dsk` — единый mutable root-диск. Каждый `cp` копирует указанный файл в один и тот же disk image.

## Разработка

Если у вас есть идеи, предложения или исправления, буду благодарен любым [issue](https://github.com/notKitory/fuzix-sh/issues) или [pull request](https://github.com/notKitory/fuzix-sh/pulls).
