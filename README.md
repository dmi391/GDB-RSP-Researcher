# GDB-RSP-Researcher

Simple GDB-server for GDB RSP-protocol research.

Implemented:

* Connection to GDB-client by TCP
* Responses to RSP commands from GDB-client
* Loop imitation of target program. And interrupt it by ^C (working in two threads)

## Build and launch
    cargo run --release -- --loop
    or
    cargo run --release

## Launch
    gdb-rsp-researcher.exe --loop
    or
    gdb-rsp-researcher.exe

## Arguments
`--loop` or `-l` : Loop imitation of target program execution (optional)

## Working with GDB-client
Launch GDB-client with path to elf-file as parameter:

    /path-to-gdb/gdb /path-to-elf/file.elf

In GDB CLI:

Turn on RSP debug mode (optional):

    (gdb) set debug remote 1

Connect to GDB-server:

    (gdb) target remote localhost:9999
        ...
        initial dialog without loading elf sections

Halt the core:

    (gdb) monitor reset halt
        or
    (gdb) monitor reset init

Load sections from elf:

    (gdb) load /path-to-elf/file.elf
        or
    (gdb) load

__It is ready for work__

Set breakpoint:

    (gdb) break function_name
        or
    (gdb) b function_name
        or
    (gdb) b linenum
        or
    (gdb) b filename:linenum
        or
    (gdb) b 0xaddress

Set watchpoint:

    (gdb) watch var

Information about breakpoints and watchpoints:

    (gdb) info break

Launch execution:

    (gdb) continue
        or
    (gdb) c

Interrupt execution: Ctrl+C

Execute one line:

    (gdb) step
        or
    (gdb) s

Execute one instruction:

    (gdb) stepi
        or
    (gdb) si

Examining memory:

    (gdb) x addr
    (gdb) x var
    (gdb) print/x var
    (gdb) x/8b 0x10030000 – read 8 bytes from address
    (gdb) x/8c 0x10030000 - 8 chars
    (gdb) x/8h 0x10030000 – 8 half-words (2 bytes)
    (gdb) x/8w 0x10030000 – 8 words (4 bytes)

Write register:

    (gdb) set $t0 = main
        or
    (gdb) set $t0 = 0xaddress

Read register:

    (gdb) print $t0
        or
    (gdb) p/x $t0

Read all registers:

    (gdb) info registers
        or
    (gdb) info all-registers
