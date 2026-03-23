If you only plan on running Kawari locally and not putting it behind a reverse proxy, we need to add the domains to your machine's hosts file.

```
127.0.0.1 admin.ffxiv.localhost
127.0.0.1 ffxiv.localhost
127.0.0.1 launcher.ffxiv.localhost
127.0.0.1 config-dl.ffxiv.localhost
127.0.0.1 frontier.ffxiv.localhost
127.0.0.1 patch.ffxiv.localhost
127.0.0.1 ffxiv-login.square.localhost
127.0.0.1 dctravel.ffxiv.localhost
```

This file is located under `C:\Windows\System32\Drivers\etc\hosts` on Windows, and `/etc/hosts` on macOS and Linux.

On Windows, run `ipconfig /flushdns` if you get network issues while trying to launch. This ensures the modifications to your hosts file is actually used. If you get a version check failed message, it means the patch servers are unreachable.
