# Clipshare

Copy on any machine, paste on any machine. Clipshare is your universal shared clipboard you can use just like the native-one, anywhere.

## Core principles

There are quite a few truly awesome data sharing solutions around, such as [Croc,](https://github.com/schollz/croc) [LocalSend](https://github.com/localsend/localsend) or [Syncthing.](https://github.com/syncthing/syncthing) And while there is a certain functional overlap with all of them, Clipshare brings few distinctive features I have always been missing in a single product.

### Direct access

If I want a shared clipboard, it has to work like a clipboard! Single shortcut for copy, single shortcut for paste, even if it means doing some hacky workarounds. Copying data around should be a seamless process, I don't want to be messing with dialogs or terminal commands for inserting my data, choosing the recipient, protection etc. I just want to copy on one computer and paste on another.

### Nothing about me without me

Sharing data is cool, but just like standard copying, I want to have it under control. I.E. the shared clipboard is only ever accessed when I decide to do so. No automatic syncing, I don't want my data to be arbitrarily flowing who knows where, or my computer getting filled with whoever knows what. Even with protection mechanisms in place, I want all transfers to occur only when they're necessary and when I actively do something to invoke them.

### End to end encryption, zero knowledge servers

When I copy any data, I would like them to appear in the decrypted form only at the target location. Not anywhere else on the road! And also, there shouldn't be any trust put into servers I use. The clients should be designed in such a way that even if I use explicitly malicious server instances, there should be no risk of leaking data.

## Welcome Clipshare

All you need to do to access you shared clipboard is to press Clipshare's copy shortcut. The program will first emulate a Ctrl+C key press, copying your text selection to the system clipboard, where it's read by Clipshare while the original content is reverted. Then, Clipshare encrypts the text using AES 256 GCM, and uploads it to the server, the Argon2id v19 hash (resistant against brute-force attacks) of your encryption password is used as your unique identifier.

On a different machine, you press Clipshare's paste shortcut, first the program downloads the shared clipboard content and decrypts it, then it's copied to the system clipboard and Ctrl+V key press is emulated, pasting into whatever application are you using. Content of the system clipboard is rolled back again.

you can do this as many times as you like, the server will keep the content for serverside-configurable amount of time, 5 minutes by default. Right now, only text copying is supported, though files and directories are certainly on the roadmap. The server also sets the max size per shared clipboard, which is 5 MB by default, this should suffice even for long texts.

In the configuration of your clients (config.toml read either from the program directory or the system's native config dir/clipshare, see the repo for the recommended default), you can configure as many clipboards as you want. You can use them as a standard multiclipboard, but you can also scope access in this way, having separate clipboards with separate encryption passwords for your personal computers, for your development VMs, or you can even setup clipboards for sharing data with your friends.

Linux and Windows platforms are supported at the moment.

### Sync mode

A little terminology clearup, Clipshare is designed in such a way that you shouldn't need to use Ctrl+C or Ctrl+V shortcuts, the program emulates them for you automatically. However, there are times when you may want to just copy what you have in your system clipboard or to paste into your system clipboard, for example because the app you're using implements non-standard copy/paste shortcuts.

Sync mode exists for this reason. It just *synchronises* the states of your system and shared clipboard, in the direction of normal operation i.e. copying system -> shared, pasting shared -> system. The daemon command of Clipshare supports configuring shortcuts for sync copy / paste, using copy and paste Clipshare commands gives you a flag for activating sync mode.

## Installation and usage

First, get the Clipshare binary, either via the Github Releases or compile from source as described below. Put it into a stable place, like /usr/local/bin on Linux or C:\\Program files\\Clipshare\\clipshare on Windows.

Next, download the [configuration file](https://github.com/RastislavKish/clipshare/blob/main/config.toml) and change it to your liking, most importantly, change the password of the Primary clipboard to a long, random string. Then, place the configuration file either next to the executable, or to your OS specific config dir, like ~/.config/clipshare on Linux.

When done, the most convenient thing is to make Clipshare run after the system start. Among the commands that are run, include "clipshare daemon", on Linux, or, "C:\\Program files\\clipshare\\clipshare.exe daemon" on Windows. After the program is run, pressing your configured shortcuts should trigger Clipshare notifications.

### A security notice

Clipshare pays great attention on securing your data during the transport from one computer to another. However, there is not yet a particular emphasis on hardware security of the clients, like erasing the clipboard content from memory after use, properly zeroing encryption keys etc. Keep it in mind when working with sensitive data, just like you do with your system clipboard.

## Build from source

### Dependencies

* The [Rust programming language](https://www.rust-lang.org/tools/install)
* libxdo-dev on Linux

### Building

```
git clone https://github.com/RastislavKish/clipshare
cd clipshare
cd desktop_client
cargo build --release -q
# You can build the server in the same way
cd ../server
cargo build --release -q
```

## Self hosting an instance via Docker

You can use docker to self-host your own instance of Clipshare server. This is an example compose.yaml file for docker compose:

```
version: "3.8"

services:
    clipshare:
        container_name: clipshare
        image: rastislavkish/clipshare:latest
        environment:
            - CERT_DIR=/etc/letsencrypt/live/yourdodmain.com
            - REDIS_HOST=redis://clipshare-redis
        ports:
            - "3127:3127"
        networks:
            - clipsharenet
        volumes:
            - "certs:/etc/letsencrypt:ro"
        depends_on:
            - clipshare-redis
    clipshare-redis:
        container_name: clipshare-redis
        image: redis:6.2.5-alpine
        command: redis-server
        networks:
            - clipsharenet
networks:
    commonnet:
volumes:
    certs:
```

Use ```sudo docker compose up``` to get things running.

### Environment variables

You can configure your Clipshare server using these environment variables:

Name | Description | efault
--- | --- | ---
REDIS_HOST | The redis host to use in format redis://hostname | redis://127.0.0.1
CERT_DIR | The directory containing fullchain.pem and privkey.pem certificates (this variable is mandatory) | None
SERVER_PORT | The server port to use | 3127
RESTRICTED_TO | A comma separated list of IDDs allowed to use the service. If not set, any clipboard ID can be used. | None
MAX_CLIPBOARD_COUNT | The maximum number of clipboards allowed to exist at the same time | 10000
MAX_USED_SPACE | The maximum space all clipboards can use in total | 500M
CLIPBOARD_CONTENT_EXPIRATION_TIME | The time period for which the server keeps a clipboard record | 5M (meaning 5 min)
CLIPBOARD_CONTENT_MAX_SIZE | The max size a single clipboard can have | 5M

### A note on SSL

SSL is a critical part of Clipshare's security model. Without this layer of protection, an attacker couldn't read your data, but could capture the ID of your clipboard, and use it to wipe out its content and cause other obstructions. Therefore, you should configure SSL certificates for your server, Clipshare clients won't connect to anything that's not protected.

## License

Copyright (C) 2023 Rastislav Kish

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, version 3.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.

