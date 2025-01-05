# Matrix-keyboard

Matrix-keyboard is a Pi Pico + small PCB designed to connect to the vending machine matrix keypad header and emulate a USB keyboard for feeding these input events to the Pi.

## Description

The vending machine presents a 3 x 8 matrix of key buttons.

The firmware is written in Rust and based on the Embassy Async framework.

It also provides a 5V level shifted I2C interface to allow it to drive the I2C LCD display.

KiCad project files are supplied for the small PCB which the Pico is mounted to.

Postcard-RPC endpoints are used to allow the host to set the text to display on the LCD (which will autoscroll if necessary).
State changes of the service mode switch are also broadcast to the host as a Postcard-RPC topic.
