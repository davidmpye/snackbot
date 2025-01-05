# Matrix-keyboard

Matrix-keyboard is a Pi Pico + small PCB designed to the vending machine keypad inputs and emulate a USB keyboard
for feeding these input events to the Pi.

## Description

The vending machine keypad interface is a 3 x 8 matrix of key buttons.
This firmware appears as a USB HID Keyboard and the matrix keypad button presses are shared to the USB host as keyboard button keypresses.

The firmware also provides postcard-rpc endpoints (see keyboard-icd subfolder) to allow the Host to set the text and control the backlight of an optionally attached I2C LCD (the schematic includes 3v3->5v level shifting to ensure reliable operation with 5V-driven LCDs)