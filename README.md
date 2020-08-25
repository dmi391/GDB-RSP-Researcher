# GDB-RSP-Researcher

Simple GDB-server for GDB RSP-protocol research.
Простой GDB-server для исследования GDB RSP-протокола.
Реализовано:
* Подключение к GDB-клиенту по TCP
* Ответы на RSP команды от GDB-клиента
* Имитация зацикливания исполняемой программы и ее прерывания по ^C (работа в двух потоках)

## Сборка и запуск:
	cargo run --release -- --loop
	cargo run --release

## Запуск:
	gdb-rsp-researcher.exe --loop
	gdb-rsp-researcher.exe

## Параметры:
	--loop or -l : Имитация зацикливания исполняемой программы. Опционально.

## Работа с GDB:
Запустить GDB и передать ему параметр файл .elf:
	/pass-to-gdb/gdb /pass-to-elf/file.elf

В GDB-консоле:

Включить отладочный режим для RSP (необязательно):
	(gdb) set debug remote 1

Подключиться к GDB-серверу:
	(gdb) target remote localhost:9999
	Произойдет инициирующий диалог (без прошивки).

Остановить ядро:
	(gdb) monitor reset halt
	или
	(gdb) monitor reset init

Выполнить загрузку секций из файла .elf:
	(gdb) load /pass-to-elf/file.elf
	или
	(gdb) load


Можно работать

Установить breakpoint:
	(gdb) break main
	или
	(gdb) b main
	или
	(gdb) b номер_строки
	или
	(gdb) b 0xадресHEX

Установить watchpoint:
	(gdb) watch var

Посмотреть информацию об установленных breakpoint и watchpoint:
	(gdb) info break

Запустить исполнение:
	(gdb) continue
	(gdb) c

Прервать исполнение: Ctrl+C

Выполнить одну строку:
	(gdb) step
	(gdb) s

Выполнить одну инструкцию:
	(gdb) stepi
	(gdb) si

Чтение памяти:
	(gdb) x addr
	(gdb) x var
	(gdb) print/x var
	(gdb) x/8b 0x10030000 – считать 8 байт из памяти
	(gdb) x/8c 0x10030000 - 8 символов
	(gdb) x/8h 0x10030000 – 8 полуслов (2 байта)
	(gdb) x/8w 0x10030000 – 8 слов (4 байта)

Запись регистра:
	(gdb) set $pc = main
	или
	(gdb) set $pc = 0xадресHEX

Чтение регистра:
	(gdb) print $pc
	(gdb) p/x $pc

Чтение всех регистров:
	(gdb) info registers
	и
	(gdb) info all-registers
