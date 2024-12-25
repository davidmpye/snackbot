# Matrix-keyboard

Matrix-keyboard is a Pi Pico + small PCB designed to the vending machine keypad inputs and emulate a USB keyboard
for feeding these input events to the Pi.

## Description

The vending machine is a 3 x 8 matrix of key buttons.

The firmware is written in Rust and based on the Embassy Async framework.

It also provides a 5V level shifted I2C interface to allow it to drive the I2C LCD display.

KiCad project files are supplied for the small PCB which the Pico is mounted to.
