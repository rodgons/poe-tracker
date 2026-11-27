# Third-party licences

POE Tracker is released under GPL-3.0 (see `LICENSE`). It bundles data from the projects below.

## exile-leveling

The Leveling guide's route (`src-tauri/data/route.json`) is derived from
[exile-leveling](https://github.com/HeartofPhos/exile-leveling) by HeartofPhos: its campaign route and
its area, quest and gem data. The exact commit is recorded in the file's `source` field.

```
MIT License

Copyright (c) 2025 HeartofPhos

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## poewiki

exile-leveling's quest vendor rewards (which gems each quest's NPC sells afterwards) come from the
[Path of Exile Wiki](https://www.poewiki.net/), whose content is available under
[CC BY-NC-SA 3.0](https://creativecommons.org/licenses/by-nc-sa/3.0/).

Path of Exile is a trademark of Grinding Gear Games. Area, quest and gem names belong to Grinding Gear Games.

## Exile-UI

The Zone layouts (`src-tauri/data/layouts.json`) are simplified line drawings traced from the
Act-Decoder pictures of [Exile-UI](https://github.com/Lailloken/Exile-UI) by Lailloken, which
also gave us which zones have known layouts and how they branch. The markers are our own; no
picture or icon from Exile-UI is bundled. The exact commit is recorded in the file's `source` field.

```
MIT License

Copyright (c) 2022 Lailloken

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Cinzel and Source Sans 3 (fonts)

The app's type is bundled in `ui/fonts/`: [Cinzel](https://github.com/NDISCOVER/Cinzel) for headings
(Copyright 2020 The Cinzel Project Authors) and [Source Sans 3](https://github.com/adobe-fonts/source-sans)
for everything else (Copyright 2010-2020 Adobe, with Reserved Font Name 'Source'). Both are licensed under
the SIL Open Font License, Version 1.1; the full licence texts ship beside the fonts as
`ui/fonts/OFL-Cinzel.txt` and `ui/fonts/OFL-SourceSans3.txt`.
