Heimdall
========

Heimdall is NFC - based access system for [Progressbar hackerspace](https://progressbar.sk). It uses PN532 connected to Raspberry Pi which controls electronic lock via relay. Due to need for security and reliability it's written in Rust programming language and strives to use it's strong type system to achieve this goals.

This is alpha version and it comes with varios limitations. However, it's already useful for opening the door. Additional features need to be developed along with other crates that were created for this project.

What works:

- [X] Adding tags to database
- [X] Scanning and authenticating tags
- [X] Opening the door

TODO

- [ ] More robust main loop (remove remaining panics)
- [ ] Improve error handling
- [ ] Logging basic info to database (this will be limited to single enter/leave events to maintain privacy)
- [ ] Automatic mode
- [ ] Rewards for checkout
- [ ] Punishments/bans for late payment
- [ ] Other authentication methods (mainly smartphones)
- [ ] Async IO handling
- [ ] Interrupt driven waiting

Non-related to this code

- [ ] Automatic launch of heimdall after start of RPi (power on)
- [ ] Watchdog
- [ ] Power backup
- [ ] Interfacing with other hardware in Progressbar (Outside door, lights...)
- [ ] Interactive display/LEDs/speaker/etc at Progressbar door (outside)

If anyone's willing to help, PRs are very much appreciated.
