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

- Windows 10/11
- macOS 12 Monterey or later

It's technically possible to run other OS's as well(such as Linux), but that hasn't been tested.
