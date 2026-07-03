Create a `config.yaml` in the Kawari directory. At a minimum, you have to specify the path to the game directory:

```yaml
filesystem:
    game_path: C:\Program Files (x86)\SquareEnix\FINAL FANTASY XIV - A Realm Reborn\game
```

More configuration options can be found in `config.rs`, such as changing the ports services run on. If you plan on just running it locally for yourself, you can begin running the servers.

> [!NOTE]
> The World server may fail to start if the game isn't up-to-date. This can be temporarily ignored, since the Patch and Login servers will still run to update your client.
