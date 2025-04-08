# #!/usr/bin/env bash

# # set -e

# BUILD_MODE=""
# case "$1" in
# "" | "release")
#     bash scripts/build.sh
#     BUILD_MODE="release"
#     ;;
# "debug")
#     bash scripts/build.sh debug
#     BUILD_MODE="debug"
#     ;;
# *)
#     echo "Wrong argument. Only \"debug\"/\"release\" arguments are supported"
#     exit 1
#     ;;
# esac

# web-flash --chip esp32 target/xtensa-esp32-none-elf/${BUILD_MODE}/aula_1
# # esptool --chip esp32 -p COM5 -b 1500000 --before default_reset --after hard_reset write_flash --flash_mode dio --flash_freq 40m --flash_size 4MB 0x10000 target/xtensa-esp32-none-elf/release/aula_1
web-flash --chip esp32 target/xtensa-esp32-none-elf/release/aula_1
