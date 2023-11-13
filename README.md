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

In the configuration of your clients (config.toml read either from the current working directory or the system's native config dir/clipshare), you can configure as many clipboards as you want. You can use them as a standard multiclipboard, but you can also scope access in this way, having separate clipboards with separate encryption passwords for your personal computers, for your development VMs, or you can even setup clipboards for sharing data with your friends.

Linux and Windows platforms are supported at the moment.

## Build

### Dependencies

* The [Rust programming language](https://www.rust-lang.org/tools/install)

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

## Usage

Run

```
clipshare daemon
```

To have clipshare running in the background, listening to the configured shortcuts.

Note this doesn't seem to work just yet on Windows (feedback appreciated), what should be addressed soon.

Another approach is to use direct commands:

```
clipshare copy
```

and

```
clipshare paste
```

For setting up one's own shortcut handling, for example through Autohotkey on Windows. See the --help flag for an overview of supported settings.

### A security notice

Clipshare pays great attention on securing your data during the transport from one computer to another. However, there is not yet a particular emphasis on hardware security of the clients, like erasing the clipboard content from memory after use, properly zeroing encryption keys etc. Keep it in mind when working with sensitive data, just like you do with your system clipboard.

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

