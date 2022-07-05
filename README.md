## Overview

This is the third variant of my Raspberry Pi based door openers. It allows authorized users to open the door by sending commands to a dedicated Telegram bot.

In contrast to previous variants, the door is opened purely electrically: An analogue modem is used to dial a special house-internal number that triggers the telephone installation to apply alternating voltage to the door opener, which in turn allows opening the door by simply pushing against it.

The Rust based software that is running on the Raspberry Pi is hosted in this repository.

```
┌──────────────┐       ┌──────────┐        ┌──────────────┐
│ Raspberry Pi ├──────►│ Analogue ├───────►│ Telephone    │
└────┬─────────┘       │  Modem   │        │ Installation │
     │  ▲              └──────────┘        └───────┬──────┘
     │  │                                          │
     ▼  │                                          │
  ┌─────┴────┐                                     ▼
  │ Telegram │                              ┌─────────────┐
  │ Bot API  │                              │ Door opener │
  └──────────┘                              └─────────────┘
```

## Software

The service that is running on the Raspberry Pi v1 is written in Rust and cross compiled for ARMv6 architecture. [Teloxide](https://github.com/teloxide/teloxide) is used for the Telegram Bot functionality and [tokio-serial](https://github.com/berkowski/tokio-serial) to send [AT commands](https://en.wikipedia.org/wiki/Hayes_command_set) via the RS232 interface to the modem.

The Telegram bot offers various commands that are split into user and maintainer commands:

![bot_commands](https://drive.google.com/uc?export=view&id=1QbwUIF7xqfZuQvVevaQIjwLkGcB6_Mok)

In addition to the standard user commands, the bot maintainer can add or remove users that are authorized to issue the `/open_door` command. Additionally, with `/send_modem_cmd`, the maintainer can send direct AT commands to the modem, which can be used for testing purposes (or even to call someone from the modem).

### Cross compiling Rust for ARMv6

The Raspberry Pi v1 has an ARMv6 CPU, so the corresponding target is `arm-unknown-linux-gnueabihf`. For cross compilation, I used a [cross](https://github.com/cross-rs/cross) based Docker image that already contains the required toolchain.

Building for a different architecture is then quite straightforward:
```sh
TARGET=arm-unknown-linux-gnueabihf
rustup target add $TARGET
cargo build --release --target $TARGET
```

The only problem that I encountered were related to OpenSSL since suppport for it [had been removed](https://github.com/cross-rs/cross/issues/229) from cross. However, it turned out that one can simply build a vendored copy of OpenSSL by adding
```
openssl = { version = "0.10", features = ["vendored"] }
```
as a dependency.

Once compiled, the binary was `scp`-ed to the Raspberry, where it is automatically started by a systemd service.

## Q & A

- > What was the motivation for this project?

  I had friends visiting and not enough physical keys. Besides this, doing such a project is also a lot of fun.

- > Why Rust? Wouldn't a simple Python script have sufficed?

  Definitly. However, main motivation to use Rust was to learn it and to see how easy it can be cross compiled.

- > What about security?

  That's indeed a bit of a concern but not something I really worry about. I think I can rely on Telegram's security here and once my friends are gone, I will remove the door opener again.

- > Any issues / future improvements?

  Right now, some parts (e.g. the AT commands to open the door) are rather hardcoded. However, I doubt that anyone else has a similar setup at home, so generalizing it is probably a waste of time.

  What would be nice having, is to record a Telegram voice message and let the bot call a number and deliver that voice message via phone.

## Previous door opener variants



### V1

Youtube video: https://www.youtube.com/watch?v=-QZS3CHGylk

This one was made back in 2014. It could open both the house door and the apartment door. The house door was opened by closing a relais which would activate the electric door opener. The apartment door was opened mechanically using a stepper motor and a wooden lever.

Actual door opening would be triggered by executing a Python script on the raspberry Pi that was connected to the PCB with relais and motor flyback diodes. I wrote a dedicated Android App for that which used public key authentication to connect to the Pi (every user had their own certificate).


### V2

Youtube video: https://www.youtube.com/watch?v=_Ve_0zbm1uQ

This one was quickly hacked together in winter 2017. The hardware and door opening mechanism is pretty much the same as the previous version (except that the relais is not used since only one door had to be opened).

Instead of an Android App, I opted for a simple website hosted on the Pi that would utilize client certificate authentication (every user got their own certificate). Additionally, the door could be opened by sending a message in a authorized Telegram group (Telegram bot).

