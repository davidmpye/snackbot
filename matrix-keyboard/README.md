# Matrix-keyboard

Matrix-keyboard is a Pi Pico + small PCB designed to connect to the vending machine matrix keypad header and emulate a USB keyboard for feeding these input events to the Pi.

## Description
The vending machine keypad interface is a 3 x 8 matrix of key buttons.
This firmware appears as a USB HID Keyboard and the matrix keypad button presses are shared to the USB host as keyboard button keypresses.

The firmware also provides postcard-rpc endpoints (see keyboard-icd subfolder) to allow the Host to set the text and control the backlight of an optionally attached I2C LCD (the schematic includes 3v3->5v level shifting to ensure reliable operation with 5V-driven LCDs

![Matrix keyboard picture](https://private-user-images.githubusercontent.com/2261985/400223531-3191ab20-acc9-444a-97f4-25ef8820f795.jpg?jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJnaXRodWIuY29tIiwiYXVkIjoicmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbSIsImtleSI6ImtleTUiLCJleHAiOjE3MzYxMDA0MzYsIm5iZiI6MTczNjEwMDEzNiwicGF0aCI6Ii8yMjYxOTg1LzQwMDIyMzUzMS0zMTkxYWIyMC1hY2M5LTQ0NGEtOTdmNC0yNWVmODgyMGY3OTUuanBnP1gtQW16LUFsZ29yaXRobT1BV1M0LUhNQUMtU0hBMjU2JlgtQW16LUNyZWRlbnRpYWw9QUtJQVZDT0RZTFNBNTNQUUs0WkElMkYyMDI1MDEwNSUyRnVzLWVhc3QtMSUyRnMzJTJGYXdzNF9yZXF1ZXN0JlgtQW16LURhdGU9MjAyNTAxMDVUMTgwMjE2WiZYLUFtei1FeHBpcmVzPTMwMCZYLUFtei1TaWduYXR1cmU9Yzg4ZTJkZDhmNmI2NzI0YzIyZTFjOTJjZjUzNzYxODY0MGI2NDdlNWEwMTFjMmZhMjk5MTRlNDcxYzIzOWU1ZSZYLUFtei1TaWduZWRIZWFkZXJzPWhvc3QifQ.oYjQgVpXzc-qRg_nyyYltGReJC5QKtszokIlRf1qOLs)
