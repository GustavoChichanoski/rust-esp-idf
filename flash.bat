esptool --chip esp32 elf2image .\target\xtensa-esp32-none-elf\release\ciadiesel-rust-esp-idf
esptool erase_flash
esptool --chip esp32 --port COM5 -b 1500000 --before default_reset --after hard_reset write_flash --flash_mode dio --flash_freq 80m --flash_size detect 0x1000 bootloader/bootloader.bin 0x8000 bootloader/partitions.bin 0x10000 target/xtensa-esp32-none-elf/release/ciadiesel-rust-esp-idf.bin
