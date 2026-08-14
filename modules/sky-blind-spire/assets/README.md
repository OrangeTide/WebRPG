# Assets for The Sky-Blind Spire

This directory is empty on purpose.

The Sky-Blind Spire is (c) 2016 Michael Prescott and licensed CC BY-NC 3.0.
That licence is not compatible with this repository's MIT-0 licence, so the
module's art is not committed here. The maps still install; they come up as
empty grids and the GM attaches the background.

To complete the maps, run:

```sh
scripts/fetch-spire-assets.sh
```

That downloads the one-page PDF from trilemma.com and renders it into this
directory under the names `maps.json` expects. If you already have the images
extracted somewhere, point it at them instead and it copies rather than
downloads:

```sh
SPIRE_SOURCE=~/path/to/images scripts/fetch-spire-assets.sh
```

The rendered map is the whole adventure page, room key text and all, which
suits the GM-only map it backs. Crop it yourself if you want the isometric
drawing alone.

Everything in this directory except this README is gitignored, so a fetch never
ends up committed.

The names `maps.json` expects:

| File | Map |
|------|-----|
| `24-Sky-Blind-Spire-Map.png` | The Spire (GM only) |
| `24-Towertop.png` | Rooftop altar |
| `tower-elevation.png` | Tower elevation (players) |

Anything present here when the module is installed is ingested into media
storage and attached to its map. Anything missing is reported in the install
result, and the map is created without a background.

The tower elevation is not Prescott's art. Build it from
`player/handouts/tower-elevation.typ` in the session materials, or draw your
own: four faces, one window per floor, chains between them, no room numbers.
