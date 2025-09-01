# Sapphire Proxy

Kawari can be used as a "proxy server" for [Sapphire](https://github.com/SapphireServer/Sapphire) as they don't support anything but their own launcher. Kawari supports a lot more such as the official launcher and [Astra](https://xiv.zone/software/astra).

## Setup

First, you need to add the following to your `config.yaml`:

```yaml
enable_sapphire_proxy: true
sapphire_api_server: "127.0.0.1:54995"
```

Of course you need point `sapphire_api_server` to your API server, as it's configured in Sapphire already. Note that your lobby config needs to be be configured to point to your Sapphire server as well, or else clients will be led to the wrong port:

```yaml
lobby:
    port: 54994
```

Then you're all done, and Kawari will forward login calls to Sapphire! You can also create users in the web interface, but you cannot manage accounts.

Additionally, you don't need to run the following servers as they'll be useless in this mode:
* Admin
* Data Center Travel
* Frontier
* Lobby
* Save Data Bank
* World
