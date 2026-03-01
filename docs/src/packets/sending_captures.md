# Sending Captures

If you're asked to send a capture to someone, refer to this tutorial to ensure it's done safely and securely.

## Finding the `.cfcap` file

Chronofoil will save all captures to your hard drive, by default under `C:/Users/<username>/AppData/Local/chronofoil/`. You can configure this path under Chronofoil settings. Inside the folder will be a bunch of `.cfcap` files and they're dated by when you logged out.

## Censoring the `.cfcap` file

Do *not* send your capture as a `.cfcap` file to anybody, including us. This is because packets can contain sensitive information you probably do not want to share, and said fields [are enumerated by the Chronofoil Project here](https://github.com/ProjectChronofoil/Chronofoil.Plugin?tab=readme-ov-file#isnt-there-sensitive-stuff-in-those-packets).

Censoring a capture is easy, but first you need to [download Chronofoil.CLI for your platform](https://github.com/ProjectChronofoil/Chronofoil.CLI/releases/latest). Once downloaded, run the following command in your system's terminal. Of course, substitute the program to where it's actually located:

```bash
./cfcli-1.1.0-linux-x64 cf censor --capture mycapture.cfcap
```

Once the command completes its operation, you will find a `.ccfcap` file where the `.cfcap` file is. This file is the one you should transport.

## Sending it to someone

Even though you have a censored capture, you should still be careful to whom and how you send this file. Even with best-effort censoring, **these captures can be used to personally identify your account and character**.

If you're sending the file over the internet, **prefer encrypted options like Signal or Matrix**. Avoid public file shares too, and use private options wherever possible.
