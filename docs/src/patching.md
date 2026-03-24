# Patching

We support serving patch files to clients like [Astra](https://xiv.zone/software/astra). We will assume you already have already acquired the patch files and can't point on where to find them.

First, create a folder named "patches" next to the Kawari executables. If you prefer a different location, this can be customized in the config:

```yaml
patch:
    patches_location: C:\My super cool patches
```

Regardless, you will need to ensure the folder structure is set up as follows:

```
patches/
    boot /
        XYZ.patch
        ...
    game /
        ABC.patch
        ...
```

If the patches were organized into folders like "2d2a390f" or "48eca647" you will need to rename these to "boot" and "game" respectively.
