# TDM: التراخيص — TDM: licenses

<div dir="rtl">

## TDM

TDM برنامج حر بترخيص GPL-3.0-or-later (ملف `LICENSE` بجانب هذا الملف). ولا يطلب الترخيص أي موافقة، ولا يمنع الهندسة العكسية.

## الأدوات التي ينزّلها TDM لتحميل الفيديو

لا تأتي هذه الأدوات مع المثبّت. ينزّلها TDM عند أول تحميل فيديو، بعد موافقتك، ويتحقق من بصمة كل ملف قبل كل تشغيل.

### FFmpeg (ffmpeg وffprobe)

يستعمل هذا البرنامج شيفرة من مشروع FFmpeg بترخيص LGPL 2.1 أو أحدث (نصه في `LGPL-2.1.txt` بجانب هذا الملف)، ويمكن تنزيل مصدرها من [إصدار tools-ffmpeg-8.1.3-1](https://github.com/tawaswlsy-stack/TDM/releases/tag/tools-ffmpeg-8.1.3-1)، ومعه طريقة البناء كاملة.

- النسخة: FFmpeg 8.1.3، من بناء TDM (`8.1.3-1`)، من المصدر الرسمي دون أي تعديل، وبلا `--enable-gpl` ولا `--enable-nonfree` ولا أي مكتبة خارجية.
- FFmpeg ملك أصحابه، وليس ملكًا لTDM: [ffmpeg.org](https://ffmpeg.org).
- حقوق النشر © مطوّرو FFmpeg. ويأتي FFmpeg **بلا أي ضمان** (انظر `LGPL-2.1.txt`).

### yt-dlp

الملف التنفيذي الذي ينزّله TDM بترخيص GPLv3 أو أحدث (والكود المصدري بترخيص Unlicense). وهو ملك أصحابه: [github.com/yt-dlp/yt-dlp](https://github.com/yt-dlp/yt-dlp).

### QuickJS-NG

بترخيص MIT، لتحميل فيديو يوتيوب فقط إن لم يوجد Deno أو Node، ويعمل في صندوق مغلق. وهو ملك أصحابه: [github.com/quickjs-ng/quickjs](https://github.com/quickjs-ng/quickjs).

## المكتبات داخل البرنامج

تراخيص المكتبات المبنية داخل TDM (Rust وnpm)، وسبب اختيار كل منها، في [docs/DEPENDENCIES.md](https://github.com/tawaswlsy-stack/TDM/blob/main/docs/DEPENDENCIES.md).

</div>

## TDM

TDM is free software under GPL-3.0-or-later (the `LICENSE` file next to this one). The licence asks for no acceptance and does not forbid reverse engineering.

## The tools TDM downloads for video

These tools are not in the installer. TDM downloads them at the first video download, after you agree, and checks each file's fingerprint before every run.

### FFmpeg (ffmpeg and ffprobe)

This software uses code of FFmpeg licensed under the LGPLv2.1 (or later; the text is in `LGPL-2.1.txt` next to this file) and its source can be downloaded [here](https://github.com/tawaswlsy-stack/TDM/releases/tag/tools-ffmpeg-8.1.3-1), with the whole way it was built.

- Version: FFmpeg 8.1.3, TDM's build `8.1.3-1`, from the official source unchanged, without `--enable-gpl`, `--enable-nonfree` or any outside library.
- FFmpeg belongs to its authors, not to TDM: [ffmpeg.org](https://ffmpeg.org).
- Copyright © the FFmpeg developers. FFmpeg comes with **absolutely no warranty** (see `LGPL-2.1.txt`).

### yt-dlp

The executable TDM downloads is under GPLv3 or later (its source code is under the Unlicense). It belongs to its authors: [github.com/yt-dlp/yt-dlp](https://github.com/yt-dlp/yt-dlp).

### QuickJS-NG

MIT. Used only for YouTube when neither Deno nor Node is installed, in a closed sandbox. It belongs to its authors: [github.com/quickjs-ng/quickjs](https://github.com/quickjs-ng/quickjs).

## The libraries inside the program

The licences of the libraries built into TDM (Rust and npm), and why each was chosen, are in [docs/DEPENDENCIES.md](https://github.com/tawaswlsy-stack/TDM/blob/main/docs/DEPENDENCIES.md).
