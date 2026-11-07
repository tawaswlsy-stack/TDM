# تدقيق توجيه صفحات HTML في TDM

## الخلاصة

لا يصنّف التطبيق نوع الملف من الرابط قبل اختيار المحرك. إضافة الرابط يدوياً تستخدم محرك HTTP العادي، ومسار الفيديو يُفتح باختيار المستخدم لمسار الفيديو. يفحص التطبيق `Content-Type` أثناء المعاينة ليعرض تحذيراً ويمنع البدء التلقائي لصفحة HTML، لكنه لا يمنع الإضافة اليدوية إلى طابور HTTP. لذلك يمكن حفظ استجابة HTML كتنزيل ناجح، وقد تحمل امتداداً مثل `.mp4` إذا جاء هذا الامتداد من الرابط أو من `Content-Disposition`.

## الإجابات

| السؤال | النتيجة والتنفيذ الفعلي |
|---|---|
| 1. هل يصنّف الرابط قبل توجيهه إلى HTTP أو yt-dlp؟ | لا يوجد توجيه آلي عام بحسب نوع الرابط أو امتداده. نافذة إضافة الرابط تستدعي `probe` لمحرك HTTP، ثم تستخدم نتيجة `page` للواجهة فقط. ويبدأ مسار الفيديو عبر إجراء فيديو مستقل، لا عبر مصنف يختار المحرك تلقائياً. المراجع: [downloads.rs](apps/desktop/src-tauri/src/downloads.rs#L103)، [InfoWindow.svelte](apps/desktop/src/windows/InfoWindow.svelte#L92). |
| 2. هل يمكن تمرير صفحة فيديو إلى HTTP العادي؟ | نعم. إذا كانت `probe.page` صحيحة، تتجاوز النافذة البدء التلقائي وتعرض مسار الفيديو، لكن زري «ابدأ» و«أضف لاحقاً» في `AddDialog` لا يتعطلان بسبب `page`؛ تعطيلهما مرتبط بـ`blocked`. عند استخدامهما، تُضاف URL كما هي عبر `downloads.add` إلى الطابور العادي. المراجع: [InfoWindow.svelte](apps/desktop/src/windows/InfoWindow.svelte#L92)، [AddDialog.svelte](apps/desktop/src/lib/components/AddDialog.svelte#L345)، [downloads.rs](apps/desktop/src-tauri/src/downloads.rs#L185). |
| 3. هل يفشل yt-dlp ثم ينزّل التطبيق صفحة HTML الأصلية عبر HTTP؟ | لا يظهر هذا التراجع في مسار تنزيل الفيديو: فشل تشغيل yt-dlp يُعاد كخطأ، ولا يستدعي محرك HTTP لتنزيل الصفحة. يوجد تراجع مختلف أثناء **فحص** الصفحة فقط: إذا تعذر على yt-dlp قراءتها، يمكن تجربة رابط الوسيط الذي رصده المتصفح بواسطة yt-dlp أيضاً. المراجع: [video/runner.rs](crates/tdm-core/src/video/runner.rs#L378)، [video/download.rs](crates/tdm-core/src/video/download.rs#L103)، [video/download.rs](crates/tdm-core/src/video/download.rs#L188). |
| 4. هل يعتمد على Content-Type أو امتداد الرابط؟ | يتعرف فحص الصفحة على `text/html` و`application/xhtml+xml` من `Content-Type`، دون فحص محتوى الجسم. اختيار المحرك لا يعتمد على هذا الحقل أو امتداد URL. الاسم يؤخذ من `Content-Disposition` أو URL، ثم يضيف التطبيق امتداد `Content-Type` فقط إن لم يكن للاسم امتداد أصلاً؛ لذلك لا يصحح امتداداً موجوداً لا يطابق MIME. المراجع: [engine/probe.rs](crates/tdm-core/src/engine/probe.rs#L60)، [engine/probe.rs](crates/tdm-core/src/engine/probe.rs#L81)، [naming.rs](crates/tdm-core/src/naming.rs#L64). |
| 5. هل توجد حماية تمنع اعتبار HTML ملف فيديو ناجحاً؟ | لا توجد حماية عامة في محرك HTTP تمنع ذلك. دالة `is_bare_page` تُستخدم في معالجة روابط Google Drive تحديداً، وليست حاجزاً لكل استجابات HTML. كذلك لا يتحقق مسار yt-dlp في TDM من MIME أو ترويسة الملف للتأكد أنه فيديو؛ يتحقق من نجاح العملية ومسار الملف وكونه ملفاً عادياً ذي امتداد صالح. أما إضافة المتصفح فتملك فحصاً منفصلاً يرفض الصفحة عندما يكشفها `probe`. المراجع: [engine/http.rs](crates/tdm-core/src/engine/http.rs#L437)، [engine/probe.rs](crates/tdm-core/src/engine/probe.rs#L99)، [video/files.rs](crates/tdm-core/src/video/files.rs#L108)، [extension.rs](apps/desktop/src-tauri/src/extension.rs#L193). |
| 6. هل يمكن حفظ HTML بامتداد MP4 أو امتداد فيديو؟ | نعم، عبر إضافة الرابط العادية: إذا أعاد الخادم HTML لرابط ينتهي مثلاً بـ`video.mp4`، ولم يرسل اسماً مختلفاً يزيل ذلك الامتداد، يبقى الامتداد كما هو لأن إضافة امتداد MIME لا تعمل إلا عند غياب الامتداد. ويمكن أن يحدث الشيء نفسه مع اسم فيديو وارد في `Content-Disposition`. كما يثبت اختبار المحرك أن HTML العادي يُنزّل بنجاح؛ وعند غياب الامتداد يُسمّى `.html`. المراجع: [engine/download.rs](crates/tdm-core/src/engine/download.rs#L161)، [engine/probe.rs](crates/tdm-core/src/engine/probe.rs#L81)، [naming.rs](crates/tdm-core/src/naming.rs#L64)، [probe.rs](crates/tdm-core/tests/probe.rs#L127). |
| 7. ما الدوال والشروط المسؤولة؟ | إشارة الصفحة: `Probe::is_page` و`view::probe`؛ منع البدء الآلي: شرط `!probe.page` في `InfoWindow`; السماح بالإضافة اليدوية: أزرار `AddDialog` المعطلة بـ`blocked` فقط؛ إدخال HTTP: `downloads::add` ثم `add_blocking`؛ نقل HTTP: `Engine::download_choosing_until` ثم `response_name`؛ الحماية الخاصة بـDrive: شرط `drive.is_download(...) && is_bare_page(...)`؛ رفض صفحة المتصفح: `not_the_file`؛ فشل الفيديو: `VideoRunner::probe_sent` للفحص، و`VideoRunner::download_keeping`/`run` للتنزيل. المراجع موضحة في الصفوف أعلاه. |

### دلالة فحص صفحة HTML

`Probe::is_page` يعتمد على نوع المحتوى المعلن، وليس على تحليل HTML أو فحص بايتات الملف. في إضافة الرابط اليدوية، يؤدي ذلك إلى تحذير/تغيير مسار التركيز وتجاوز البدء التلقائي، لا إلى رفض التنزيل. وتُظهر اختبارات المحرك صراحةً أن استجابة `text/html` يمكن أن تُنزّل؛ إذا لم يكن للاسم امتداد، يصبح الاسم مثل `view.html`.

الاستثناء المحدود هو التعامل الخاص مع Google Drive: يلتقط محرك HTTP صفحة HTML عارية من عنوان تنزيل Drive ويحاول معالجة صفحة Drive أو يرفضها. لا يطبق هذا الشرط على روابط HTML العامة.

## مخطط مسار الرابط

```text
إدخال الرابط
    ↓
فحص HTTP للمعاينة (Content-Type → page)
    ↓
اختيار المستخدم/الواجهة، لا مصنف آلي:
    ├─ إضافة عادية → downloads.add → طابور HTTP
    │                   ↓
    │              GET والاستجابة 200
    │                   ↓
    │              الاسم من Content-Disposition/URL
    │              وإضافة امتداد MIME فقط إن غاب الامتداد
    │                   ↓
    │              حفظ الجسم؛ لا فحص عام يمنع HTML
    │
    └─ إجراء الفيديو → yt-dlp
                        ├─ فشل التنزيل → خطأ، دون تراجع إلى HTTP
                        └─ نجاح + مسار/ملف عادي/امتداد صالح → وضع الملف
```

## مراجع الشيفرة الأساسية

- معاينة الرابط وتصنيف `page`: [downloads.rs](apps/desktop/src-tauri/src/downloads.rs#L103)، [view.rs](apps/desktop/src-tauri/src/view.rs#L762)، [engine/probe.rs](crates/tdm-core/src/engine/probe.rs#L60).
- البدء التلقائي والإضافة اليدوية: [InfoWindow.svelte](apps/desktop/src/windows/InfoWindow.svelte#L92)، [AddDialog.svelte](apps/desktop/src/lib/components/AddDialog.svelte#L345)، [downloads.rs](apps/desktop/src-tauri/src/downloads.rs#L185).
- تنزيل HTTP والتسمية: [engine/download.rs](crates/tdm-core/src/engine/download.rs#L161)، [engine/probe.rs](crates/tdm-core/src/engine/probe.rs#L81)، [naming.rs](crates/tdm-core/src/naming.rs#L64).
- الاستثناء الخاص بـDrive: [engine/http.rs](crates/tdm-core/src/engine/http.rs#L437)، [engine/probe.rs](crates/tdm-core/src/engine/probe.rs#L99).
- مسار الفيديو وإضافة المتصفح: [video/runner.rs](crates/tdm-core/src/video/runner.rs#L378)، [video/download.rs](crates/tdm-core/src/video/download.rs#L103)، [video/files.rs](crates/tdm-core/src/video/files.rs#L108)، [extension.rs](apps/desktop/src-tauri/src/extension.rs#L193).
