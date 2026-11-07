# تقرير تدقيق قدرات التنزيل في TDM

## 1. أنواع المحتوى التي ينزّلها

| النوع | النتيجة | ما تنفذه الشيفرة فعليًا والمراجع |
|---|---|---|
| الملفات العادية: PDF وZIP وEXE والصور والصوت وغيرها | **مدعومة** عبر روابط HTTP/HTTPS؛ التنزيل العادي لا يشترط امتدادًا معيّنًا. | التحقق من رابط الطابور في [`check_url`](./crates/tdm-core/src/queue/store.rs#L319)، وحصر بروتوكولات HTTP في [`parse_url`](./crates/tdm-core/src/engine/http.rs#L74). |
| فيديوهات المواقع | **مدعومة بحسب ما يستطيع yt-dlp استخراجه**؛ يعرض TDM الصيغ التي يعثر عليها ثم ينزّل الاختيار. | فحص الفيديو في [`VideoRunner::probe`](./crates/tdm-core/src/video/runner.rs#L300)، وتوصيل الواجهة في [`video_probe`](./apps/desktop/src-tauri/src/video/probe.rs#L96). |
| روابط الفيديو المباشرة | **مدعومة** إذا كانت HTTP/HTTPS وتُنتج صيغة قابلة للتنزيل. يوجد مسار سريع خاص بروابط YouTube المباشرة على `googlevideo.com`؛ وما لا يناسبه يعود إلى yt-dlp. | [`checked`](./crates/tdm-core/src/video/files.rs#L95)، [`fast_url`](./crates/tdm-core/src/video/fast.rs#L31)، و[`download_keeping`](./crates/tdm-core/src/video/download.rs#L103). |
| الملفات الصوتية | **مدعومة** كملفات عادية عبر HTTP/HTTPS. كما يدعم تنزيل الصوت وحده من فيديو، بصيغة M4A/AAC. | اختيار الصوت في [`Choice::AudioOnly`](./crates/tdm-core/src/video/mod.rs#L54) وخيارات yt-dlp في [`Arg::AudioOnlyM4a`](./crates/tdm-core/src/video/args.rs#L145). |
| قوائم التشغيل | **مدعومة بحد أقصى 1000 عنصر** في الفحص؛ لا يعني ذلك تنزيل كل قائمة بلا حدود. | الحد في [`MAX_PLAYLIST_ENTRIES`](./crates/tdm-core/src/video/output.rs#L15)، ومعالجة القائمة في [`VideoRunner::probe`](./crates/tdm-core/src/video/runner.rs#L300). |
| البث الحي | **مشروط**: قد يعمل إذا استخرجه yt-dlp كصيغة قابلة للتنزيل، لكن لا يظهر في الشيفرة مسار مستقل أو ضمان مخصص لتسجيل البث الحي. | مسار الفحص والتنزيل العام هو [`probe`](./crates/tdm-core/src/video/runner.rs#L300) و[`download_keeping`](./crates/tdm-core/src/video/download.rs#L103). |
| ملفات التورنت وروابط magnet | ملف `.torrent` يمكن تنزيله **كملف عادي** إذا كان رابط HTTP/HTTPS. لا يوجد عميل تورنت أو دعم `magnet:`؛ الروابط المقبولة محصورة في HTTP/HTTPS. | [`parse_url`](./crates/tdm-core/src/engine/http.rs#L74) و[`checked`](./crates/tdm-core/src/video/files.rs#L95). |

## 2. محركات وأدوات معالجة روابط الفيديو

| المحرك/الأداة | الاستخدام الفعلي في TDM |
|---|---|
| **yt-dlp** | محرك استخراج صفحات الفيديو والصيغ، ويُشغّل كبرنامج تنفيذي منفصل: ينشئ Rust عملية مباشرة باستخدام `Command::new`، وليس استدعاء مكتبة Python داخل التطبيق. راجع [`run_unjudged`](./crates/tdm-core/src/video/download.rs#L267) و[`Tool::YtDlp`](./crates/tdm-core/src/tools.rs#L34). |
| **youtube-dl** | لا يظهر كأداة مستخدمة في مسار الفيديو الحالي. |
| **Streamlink** | لا يظهر كأداة مستخدمة في مسار الفيديو الحالي. |
| **FFmpeg وffprobe** | أداتان تنفيذيتان منفصلتان يستخدمهما TDM لدمج مسارات الصوت/الفيديو والتحويل إلى M4A عند الحاجة؛ لا تُستخدمان كمكتبة Python. تُمرر أدلتهما إلى yt-dlp في [`tool_args`](./crates/tdm-core/src/video/args.rs#L197)، ويُستدعى FFmpeg في [`fast_download`](./crates/tdm-core/src/video/fast.rs#L751). |
| **محرك TDM الداخلي** | تنزيل الملفات العادية عبر محرك HTTP في Rust، ومسار سريع لبعض روابط YouTube المباشرة. ليس محرك استخراج عامًّا بديلًا عن yt-dlp. |
| **وقت تشغيل JavaScript** | قد يستخدم yt-dlp ‏Deno أو Node الموجودين على النظام، أو QuickJS-NG الذي ينزّله TDM عند الحاجة، لمتطلبات بعض مستخرجات المواقع. هذا ليس محرك تنزيل مستقلًا. راجع [`find_js_runtime`](./crates/tdm-core/src/video/js.rs#L160). |

**Python:** لم تظهر ملفات مصدر Python ضمن المشروع الذي فُحص. تنفيذ TDM لمسار الفيديو مكتوب بـRust؛ لذلك لا توجد دالة Python داخل TDM تُستخدم هنا. yt-dlp نفسه يُجلب ويُشغّل كملف تنفيذي خارجي موثّق البصمة، وليس كمكتبة Python مستوردة في التطبيق. يوضح [`tools.rs`](./crates/tdm-core/src/tools.rs#L1) أن الأدوات التنفيذية لا تُضمّن في المثبّت، ويبيّن [`run_unjudged`](./crates/tdm-core/src/video/download.rs#L267) تشغيل yt-dlp كعملية.

## 3. البروتوكولات والحاويات

| البروتوكول/الصيغة | النتيجة | الأدلة وحدود الدعم |
|---|---|---|
| HTTP / HTTPS | **مدعومان**، وهما البروتوكولان المقبولان لروابط التنزيل والروابط الأولية للفيديو. | [`parse_url`](./crates/tdm-core/src/engine/http.rs#L74) و[`checked`](./crates/tdm-core/src/video/files.rs#L95). |
| HLS / M3U8 | **مدعوم** عبر yt-dlp. يتعرف TDM على صيغ `m3u8` ويفحص قائمة HLS المختارة؛ كما يتضمن اختبارًا ينزّل قائمة HLS ويشغّل الملف الناتج. | [`Format::is_hls`](./crates/tdm-core/src/video/mod.rs#L223)، [`check_hls`](./crates/tdm-core/src/video/runner.rs#L201)، واختبار [`hls_downloads_and_plays`](./crates/tdm-core/tests/video_real.rs#L486). |
| MPEG-DASH / MPD | **مدعوم** عبر yt-dlp. يتضمن اختبارًا يقرأ manifest بصيغة MPD ويدمج مساري الصوت والصورة في ملف واحد. | الاختبار [`dash_s_separate_picture_and_sound_are_merged_into_one_file`](./crates/tdm-core/tests/video_real.rs#L503). |
| MP4 / WebM / MKV | **حاويات ناتج مدعومة** بحسب الصيغ التي يوفرها الموقع؛ يختار المسار السريع الحاوية المناسبة عند دمج المسارات، بينما قد يحدد yt-dlp الامتداد في المسار العام. | [`container`](./crates/tdm-core/src/video/fast.rs#L76) و[`download_keeping`](./crates/tdm-core/src/video/download.rs#L103). |
| RTMP / RTMPS | **غير مدعومين كمسار تنزيل فعلي في الإعداد الحالي**: الروابط الأولية تقتصر على HTTP/HTTPS، ويُفرض على yt-dlp استخدام المنزّل الأصلي. كما أن بناء FFmpeg الموزع يقتصر على بروتوكولي `file` و`pipe`. | [`checked`](./crates/tdm-core/src/video/files.rs#L95)، [`Arg::DownloaderNative`](./crates/tdm-core/src/video/args.rs#L125)، واختبار بناء FFmpeg في [`smoke-test.sh`](./tools/ffmpeg/smoke-test.sh#L43). |
| بروتوكولات أخرى | لا تثبت الشيفرة دعمًا عامًا لبروتوكولات إضافية؛ يقبل TDM صيغ yt-dlp التي تُعاد من الفحص، لكن لا ينبغي مساواة ذلك بدعم كل بروتوكول يستطيع yt-dlp نظريًا التعامل معه. | [`VideoRunner::probe`](./crates/tdm-core/src/video/runner.rs#L300) و[`download_keeping`](./crates/tdm-core/src/video/download.rs#L103). |

## 4. هل yt-dlp مكتبة Python أم برنامج خارجي؟

**برنامج خارجي.** يتحقق TDM من ملف الأداة المُدار ثم يشغّله كعملية مستقلة (`Command::new`)، وتُمرّر له الوسائط من Rust. لا يستورد TDM حزمة yt-dlp في Python ولا يستدعي دوالها كمكتبة. المراجع: [`run_unjudged`](./crates/tdm-core/src/video/download.rs#L267)، وتعريف الأدوات وتنزيلها في [`tools.rs`](./crates/tdm-core/src/tools.rs#L34).

## 5. حالة FFmpeg

- **ليس مضمّنًا في مثبّت TDM**؛ ينزّله التطبيق ضمن أدواته بعد موافقة المستخدم، ويتحقق من بصمته قبل الاستخدام. المرجع: [`tools.rs`](./crates/tdm-core/src/tools.rs#L1) و[`video_tools_install`](./apps/desktop/src-tauri/src/video/tools.rs#L127).
- يُحفظ ضمن مجلد بيانات التطبيق `tools`؛ يحدد المسار [`folders`](./apps/desktop/src-tauri/src/video/mod.rs#L195)، ويعرض التطبيق حالة التثبيت عبر [`video_tools_status`](./apps/desktop/src-tauri/src/video/tools.rs#L65).
- **في بيئة الفحص الحالية:** لم يوجد مجلد الأدوات المُدار `%LOCALAPPDATA%\tdm\tools`، لذلك لم يُعثر على نسخة FFmpeg التي يديرها TDM. هذا لا يثبت غياب أي نسخة مستقلة أخرى على الجهاز؛ إلا أن TDM يتحقق من نسخته المُدارة ولا يعتمد على نسخة عشوائية من `PATH`.
- بناء FFmpeg الخاص بالمشروع محدود عمدًا: بروتوكولا `file` و`pipe` فقط، ومشفّر AAC؛ لذلك دوره الدمج والتحويل المحلي، لا جلب تدفقات RTMP أو HTTP. المرجع: [`smoke-test.sh`](./tools/ffmpeg/smoke-test.sh#L2).
