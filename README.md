Zomato scraper
==============

A simple Rust crate for retrievig daily menu from zomato.com

Disclaimer
----------

This is NOT official Zomato crate and I'm not affiliated with zomato.com in any
way. It's intended for personal use ONLY. Using it in commercial applications or
overloading the server might lead to legal issues! IANAL, any use of this crate
might be actually illegal, consult with lawyer first!

Usage
-----

The sole intent of this crate is to download daily menu, which you can use to
display using your desired formatting (see `examples/print-daily-menu.rs`) or
feed it into text to speech (see `examples/today-tts.rs`).

The whole crate has a trivial API consisting of one `async` function and a few
structs. Check the docs or examples.

License
-------

MITNFA
