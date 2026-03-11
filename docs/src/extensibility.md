# Extensibility

It's possible to extend Kawari in ways that don't involve touching or forking our code. For example, if you care about a quest or want to customize attack behavior that can't be upstreamed. You can also overlay modded game files if you so desire.

## Additional resources

You can overlay custom resources, basically anything under our own `resources` with the current exceptions of the `data` and `web` folders. Add a folder containing new or overlaid files to the config:

```yaml
filesystem:
  additional_resource_paths:
  - /home/user/my-custom-resources
```

Keep in mind that the folder structure has to match ours. For example, if you are adding a custom quest:

```
my-custom-resources /
  scripts /
    events /
      quest /
        000 /
          SomeQuest_00000.lua
```

## Modded files

You can overlay modded files such as Excel or level data in Kawari, which can be useful in certain situations like [traveling to new zones](../tips.md). Simply add a folder containing these files to the config:

```yaml
filesystem:
  additional_search_paths:
  - /home/youruser/my-custom-modded-files
```

The folder structure should match the game's, for example if you plan on replacing the `TerritoryType` sheet:

```
my-custom-modded-files /
  exd /
    TerritoryType.exh
    TerritoryType_1.exd
```
