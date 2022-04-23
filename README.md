# Image Compressor 2

A rust GUI image compressing program.

## Features

- Compress images in a specific directory to jpg format.
- Compress images using multiple threads.
- Archive the resulting image in various formats(for 7z format, see requirements described below).
- Delete original images if user wish.
- Save path history for next run.

## Demo

Compress original directory

![compress_demo](./demo/compress_demo.webp)

Compress and archive original directory

![compress_and_archive_demo](./demo/compress_and_archive_demo.webp)

## Supported Image Formats

Visit [image crate page](https://crates.io/crates/image). This program use [image crate](https://crates.io/crates/image) for opening a image file.

It compresses images to jpg format only!

## Supported Operating System

- Windows 10
- macOS 12 Monterey or later

It's technically possible to run other OS's as well(such as Linux, or Windows 11), but that hasn't been tested.

## Requirements for archiving with 7z format

If you want to use the feature that compress result images with directory.7z, the following conditions must be met:

#### Windows 10

1. Install [7-Zip](https://www.7-zip.org/).
2. Find 7z.exe in installed program folder and add it to path.

or just download 7z.exe file in release page, and place it next to this executable program.

#### macOS Monterey

1. Visit [7-Zip download page](https://www.7-zip.org/download.html) and download console version 7-Zip executable for macOS. 
2. Place 7zz file to home directory(which is "~").

or just download 7zz file in release page, and place it home directory.
