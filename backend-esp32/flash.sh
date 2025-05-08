#!/bin/bash

MODE=$1


if [[ "$MODE" == "build" ]]; then
  cargo build --release && espflash flash target/xtensa-esp32s3-espidf/release/catapult --monitor --flash-size 4mb
else
  espflash flash target/xtensa-esp32s3-espidf/release/catapult --monitor --flash-size 4mb
fi
